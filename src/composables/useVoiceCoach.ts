/*
===============================================================================
               SPELL COACH IA - LOCAL VOICE ENGINE (KOKORO TTS ONNX)
===============================================================================
Este arquivo é o composable responsável por fornecer voz ao coach ("useVoiceCoach").
Migramos 100% da síntese para o backend em Rust usando o Kokoro TTS local:

1. ULTRA-REALISMO E PRIVACIDADE: O modelo é gerado localmente em seu PC, sem
   necessidade de APIs ou conexões externas pós-download.
2. REPRODUÇÃO PREMIUM: Amostras brutas geradas a 24kHz são enviadas diretamente
   ao crate "rodio" sob WASAPI no backend Rust.
===============================================================================
*/

import { ref, onMounted } from "vue";
import { invoke } from "@tauri-apps/api/core";

// ---------------------------------------------------------------------------
// 1. ESTADO GLOBAL COMPARTILHADO (Shared Singleton State)
// ---------------------------------------------------------------------------
const voiceEnabled = ref(true);
const voiceVolume = ref(0.8); // 0.0 a 1.0 (Rust rodio volume)
const voiceRate = ref(1.0);   // 0.5 a 2.0 (Kokoro synthesis speed)
const selectedVoice = ref<"francisca" | "antonio" | "custom">("francisca"); // Mapeadas para pf_dora, pm_alex ou mesclagem customizada no Rust
const voiceWeights = ref({
  pf_dora: 10,   // BR Feminina
  pm_alex: 0,    // BR Masculina
  af_sky: 0,     // US Feminina
  am_adam: 0,    // US Masculina
  ef_dora: 0,    // ES Feminina
  em_alex: 0     // ES Masculina
});
const isSpeaking = ref(false);
const kokoroStatus = ref<string>("loading"); // "loading", "ready", "error: ..."
const lastVoiceError = ref<string | null>(null);


// Fila de falas para processamento sequencial caso múltiplos eventos ocorram
let speakQueue: string[] = [];
let queueProcessing = false;

export function useVoiceCoach() {
  
  // ---------------------------------------------------------------------------
  // 2. PARSER E FATIADOR INTELIGENTE (Smart Text & Sentence Splitter)
  // ---------------------------------------------------------------------------
  const cleanTextForSpeech = (text: string): string => {
    if (!text) return "";
    let clean = text;

    clean = clean.replace(/<br\s*\/?>/gi, ". ");
    clean = clean.replace(/<\/?[^>]+(>|$)/g, ""); // Remover HTML
    clean = clean.replace(/\*\*|__|\*|_/g, "");     // Remover markdown
    clean = clean.replace(/^\s*[-*+]\s+/gm, "");   // Remover marcadores
    clean = clean.replace(/#+\s+/g, "");           // Remover títulos markdown
    clean = clean.replace(/\[([^\]]+)\]/g, "$1");   // Remover colchetes

    // Siglas do LoL
    clean = clean.replace(/\bWR\b/gi, "taxa de vitória");
    clean = clean.replace(/\bKDA\b/gi, "K. D. A.");
    clean = clean.replace(/\bCS\b/gi, "farm");
    clean = clean.replace(/\bHP\b/gi, "vida");
    clean = clean.replace(/\bOP\b/gi, "forte");
    clean = clean.replace(/\bAD\b/gi, "dano físico");
    clean = clean.replace(/\bAP\b/gi, "dano mágico");
    clean = clean.replace(/\bCD\b/gi, "tempo de recarga");
    clean = clean.replace(/\bCDR\b/gi, "redução de tempo de recarga");
    clean = clean.replace(/\bCC\b/gi, "controle de grupo");
    clean = clean.replace(/\bTF\b/gi, "luta em equipe");
    clean = clean.replace(/\bTP\b/gi, "teleporte");
    clean = clean.replace(/\bJG\b/gi, "jângol");
    clean = clean.replace(/\bADC\b/gi, "adê cê");
    clean = clean.replace(/\bSUP\b/gi, "suporte");

    // Símbolos de fluxo/sequência — lidos literalmente pelo Kokoro
    clean = clean.replace(/→/g, ", depois ");
    clean = clean.replace(/←/g, ", antes ");
    clean = clean.replace(/↔/g, " ou ");
    clean = clean.replace(/[★☆▲▼●◆■□]/g, "");
    clean = clean.replace(/[/\\]/g, " ou ");

    // Emojis decorativos
    clean = clean.replace(/[\u{1F300}-\u{1F9FF}]|[\u{1F600}-\u{1F64F}]|[\u{2700}-\u{27BF}]/gu, "");
    // Limpa espaços e pontuação duplicada resultantes das substituições acima
    clean = clean.replace(/,\s*,/g, ",");
    clean = clean.replace(/\s+/g, " ");
    clean = clean.replace(/\.+/g, ".");

    return clean.trim();
  };

  // ---------------------------------------------------------------------------
  // 3. PERSISTÊNCIA DAS CONFIGURAÇÕES (Local Storage)
  // ---------------------------------------------------------------------------
  const loadSettings = () => {
    try {
      const enabled = localStorage.getItem("spell_coach_voice_enabled");
      if (enabled !== null) voiceEnabled.value = enabled === "true";

      const volume = localStorage.getItem("spell_coach_voice_volume");
      if (volume !== null) voiceVolume.value = parseFloat(volume);

      const rate = localStorage.getItem("spell_coach_voice_rate");
      if (rate !== null) voiceRate.value = parseFloat(rate);

      const savedVoice = localStorage.getItem("spell_coach_voice_selected");
      if (savedVoice === "francisca" || savedVoice === "antonio" || savedVoice === "custom") {
        selectedVoice.value = savedVoice as any;
      }

      const weights = localStorage.getItem("spell_coach_voice_weights");
      if (weights !== null) {
        try {
          voiceWeights.value = JSON.parse(weights);
        } catch (e) {
          console.warn("Erro ao carregar pesos de vozes:", e);
        }
      }
    } catch (e) {
      console.warn("Erro ao carregar configurações de voz do localStorage:", e);
    }
  };

  const saveSettings = () => {
    try {
      localStorage.setItem("spell_coach_voice_enabled", String(voiceEnabled.value));
      localStorage.setItem("spell_coach_voice_volume", String(voiceVolume.value));
      localStorage.setItem("spell_coach_voice_rate", String(voiceRate.value));
      localStorage.setItem("spell_coach_voice_selected", selectedVoice.value);
      localStorage.setItem("spell_coach_voice_weights", JSON.stringify(voiceWeights.value));
    } catch (e) {
      console.warn("Erro ao salvar configurações de voz no localStorage:", e);
    }
  };

  // ---------------------------------------------------------------------------
  // 4. MÉTODOS DE REPRODUÇÃO (Native Rust Delegate)
  // ---------------------------------------------------------------------------
  const stop = async () => {
    try {
      speakQueue = [];
      queueProcessing = false;
      isSpeaking.value = false;
      await invoke("stop_voice");
    } catch (err) {
      console.error("[useVoiceCoach:stop] Erro ao parar fala:", err);
    }
  };

  const checkKokoroStatus = async () => {
    try {
      const status = await invoke<string>("get_kokoro_status");
      kokoroStatus.value = status;
    } catch (err) {
      console.error("[useVoiceCoach] Erro ao obter status do Kokoro:", err);
      kokoroStatus.value = "error: " + err;
    }
  };

  const startStatusPolling = () => {
    checkKokoroStatus();
    const interval = setInterval(async () => {
      await checkKokoroStatus();
      if (kokoroStatus.value === "ready" || kokoroStatus.value.startsWith("error")) {
        clearInterval(interval);
      }
    }, 1000);
  };

  const getVoiceString = (): string => {
    if (selectedVoice.value === "francisca") {
      return "pf_dora";
    }
    if (selectedVoice.value === "antonio") {
      return "pm_alex";
    }

    // Filtra vozes com peso maior que zero
    const activeVoices = Object.entries(voiceWeights.value)
      .filter(([_, weight]) => weight > 0)
      .map(([voiceId, weight]) => ({ voiceId, weight }));

    if (activeVoices.length === 0) {
      return "pf_dora"; // Fallback se tudo for zero
    }

    if (activeVoices.length === 1) {
      return activeVoices[0].voiceId;
    }

    // Normaliza pesos para que a soma seja exatamente 10 (exigência do Kokoro)
    const totalRaw = activeVoices.reduce((sum, item) => sum + item.weight, 0);
    
    let normalized = activeVoices.map((item) => {
      let norm = Math.round((item.weight / totalRaw) * 10);
      return { ...item, norm };
    });

    // Corrige eventuais erros de arredondamento
    let currentSum = normalized.reduce((sum, item) => sum + item.norm, 0);
    if (currentSum !== 10) {
      let diff = 10 - currentSum;
      let sorted = [...normalized].sort((a, b) => b.norm - a.norm);
      if (sorted[0]) {
        let idx = normalized.findIndex(x => x.voiceId === sorted[0].voiceId);
        if (idx !== -1) {
          normalized[idx].norm = Math.max(1, normalized[idx].norm + diff);
        }
      }
    }

    // Garante que a soma é exatamente 10, caso contrário retorna Francisca como segurança
    currentSum = normalized.reduce((sum, item) => sum + item.norm, 0);
    if (currentSum !== 10) {
      return "pf_dora";
    }

    // Cria a string mesclada, ex: "pf_dora.5+af_sky.3+ef_dora.2"
    return normalized
      .filter(item => item.norm > 0)
      .map(item => `${item.voiceId}.${item.norm}`)
      .join("+");
  };

  // Divide o texto em sentenças para não exceder o limite do Kokoro ONNX (~200 chars)
  const splitIntoSentences = (text: string): string[] => {
    const chunks: string[] = [];
    // Divide nos pontos de pontuação mantendo o delimitador junto ao trecho anterior
    const raw = text.split(/(?<=[.!?])\s+/);
    let current = "";
    for (const part of raw) {
      if (!part.trim()) continue;
      if ((current + " " + part).trim().length > 200 && current.length > 0) {
        chunks.push(current.trim());
        current = part;
      } else {
        current = current ? current + " " + part : part;
      }
    }
    if (current.trim()) chunks.push(current.trim());
    return chunks.length > 0 ? chunks : [text];
  };

  const processQueue = async () => {
    if (queueProcessing || speakQueue.length === 0) return;
    queueProcessing = true;
    isSpeaking.value = true;

    while (speakQueue.length > 0) {
      const nextText = speakQueue.shift();
      if (!nextText) continue;

      const cleaned = cleanTextForSpeech(nextText);
      if (!cleaned) continue;

      // Se o texto for longo, divide em sentenças para evitar truncamento do Kokoro
      const sentences = cleaned.length > 200 ? splitIntoSentences(cleaned) : [cleaned];

      for (const sentence of sentences) {
        const s = sentence.trim();
        if (!s) continue;
        try {
          const voiceStr = getVoiceString();
          console.log(`[useVoiceCoach:Rust] Sintetizando trecho via Kokoro: "${s}" (Voz: ${voiceStr})`);
          await invoke("play_voice", {
            text: s,
            voice: voiceStr,
            volume: voiceVolume.value,
            speed: voiceRate.value,
          });
          lastVoiceError.value = null;
        } catch (err) {
          const errMsg = String(err);
          console.error("[useVoiceCoach:Rust] Erro ao reproduzir fala Kokoro nativa:", errMsg);
          lastVoiceError.value = errMsg;
        }
      }
    }

    queueProcessing = false;
    isSpeaking.value = false;
  };

  const speak = async (text: string) => {
    if (!voiceEnabled.value) return;
    if (kokoroStatus.value !== "ready") {
      console.warn("[useVoiceCoach] Ignorando fala: O motor Kokoro ainda está carregando ou falhou.");
      return;
    }

    // Se uma nova fala explícita for iniciada, limpamos e paramos tudo antes
    await stop();

    speakQueue.push(text);
    await processQueue();
  };

  const testVoice = async () => {
    let text = "Olá! Testando a voz local da Francisca.";
    if (selectedVoice.value === "antonio") {
      text = "Olá! Testando a voz local do Antônio.";
    } else if (selectedVoice.value === "custom") {
      text = "Olá! Testando a minha mistura personalizada de vozes neurais locais.";
    }
    
    // Liga a voz temporariamente se estiver desligada apenas para o teste
    const originalEnabled = voiceEnabled.value;
    voiceEnabled.value = true;
    
    // Bypass temporary state check to allow test even if loading
    const originalStatus = kokoroStatus.value;
    kokoroStatus.value = "ready";
    
    await speak(text);
    
    kokoroStatus.value = originalStatus;
    voiceEnabled.value = originalEnabled;
  };

  // ---------------------------------------------------------------------------
  // 5. INICIALIZAÇÃO
  // ---------------------------------------------------------------------------
  onMounted(() => {
    loadSettings();
    startStatusPolling();
  });

  return {
    voiceEnabled,
    voiceVolume,
    voiceRate,
    selectedVoice,
    voiceWeights,
    isSpeaking,
    kokoroStatus,
    lastVoiceError,
    checkKokoroStatus,
    speak,
    stop,
    testVoice,
    loadSettings,
    saveSettings,
  };
}
