import { ref, onMounted, onUnmounted } from "vue";
import { invoke } from "@tauri-apps/api/core";

// ---------------------------------------------------------------------------
// Estado global compartilhado (singleton)
// ---------------------------------------------------------------------------
const voiceEnabled = ref(true);
const voiceVolume  = ref(0.8);  // 0.0 a 1.0
const voiceRate    = ref(1.0);  // 0.5 a 2.0
const selectedVoice = ref("pf_dora"); // ID direto da API remota
const isSpeaking   = ref(false);
const isTesting    = ref(false);
const kokoroStatus = ref<string>("loading");
const lastVoiceError = ref<string | null>(null);

let speakQueue: string[] = [];
let queueProcessing = false;

export const VOICE_OPTIONS = [
  { value: "pf_dora",  label: "Francisca — Feminina BR 🇧🇷" },
  { value: "pm_alex",  label: "Antônio — Masculino BR 🇧🇷" },
  { value: "pm_santa", label: "Papai Noel — Masculino BR 🇧🇷" },
  { value: "af_sky",   label: "Sky — Feminina EUA 🇺🇸" },
  { value: "am_adam",  label: "Adam — Masculino EUA 🇺🇸" },
  { value: "ef_dora",  label: "Dora — Feminina ESP 🇪🇸" },
  { value: "em_alex",  label: "Alex — Masculino ESP 🇪🇸" },
];

export function useVoiceCoach() {

  // ---------------------------------------------------------------------------
  // Parser de texto (remove markdown, HTML, siglas do LoL, etc.)
  // ---------------------------------------------------------------------------
  const cleanTextForSpeech = (text: string): string => {
    if (!text) return "";
    let clean = text;
    clean = clean.replace(/<br\s*\/?>/gi, ". ");
    clean = clean.replace(/<\/?[^>]+(>|$)/g, "");
    clean = clean.replace(/\*\*|__|\*|_/g, "");
    clean = clean.replace(/^\s*[-*+]\s+/gm, "");
    clean = clean.replace(/#+\s+/g, "");
    clean = clean.replace(/\[([^\]]+)\]/g, "$1");
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
    clean = clean.replace(/→/g, ", depois ");
    clean = clean.replace(/←/g, ", antes ");
    clean = clean.replace(/↔/g, " ou ");
    clean = clean.replace(/[★☆▲▼●◆■□]/g, "");
    clean = clean.replace(/[/\\]/g, " ou ");
    clean = clean.replace(/[\u{1F300}-\u{1F9FF}]|[\u{1F600}-\u{1F64F}]|[\u{2700}-\u{27BF}]/gu, "");
    clean = clean.replace(/,\s*,/g, ",");
    clean = clean.replace(/\s+/g, " ");
    clean = clean.replace(/\.+/g, ".");
    return clean.trim();
  };

  // ---------------------------------------------------------------------------
  // Persistência
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
      if (savedVoice) {
        // Migra valores antigos para IDs da API
        const migration: Record<string, string> = {
          francisca: "pf_dora",
          antonio:   "pm_alex",
          custom:    "pf_dora",
        };
        selectedVoice.value = migration[savedVoice] ?? savedVoice;
      }
    } catch (e) {
      console.warn("Erro ao carregar configurações de voz:", e);
    }
  };

  const saveSettings = () => {
    try {
      localStorage.setItem("spell_coach_voice_enabled", String(voiceEnabled.value));
      localStorage.setItem("spell_coach_voice_volume",  String(voiceVolume.value));
      localStorage.setItem("spell_coach_voice_rate",    String(voiceRate.value));
      localStorage.setItem("spell_coach_voice_selected", selectedVoice.value);
    } catch (e) {
      console.warn("Erro ao salvar configurações de voz:", e);
    }
  };

  // ---------------------------------------------------------------------------
  // Reprodução
  // ---------------------------------------------------------------------------
  const stop = async () => {
    try {
      speakQueue = [];
      queueProcessing = false;
      isSpeaking.value = false;
      await invoke("stop_voice");
    } catch (err) {
      console.error("[useVoiceCoach:stop]", err);
    }
  };

  const checkKokoroStatus = async () => {
    try {
      const status = await invoke<string>("get_kokoro_status");
      kokoroStatus.value = status;
    } catch (err) {
      kokoroStatus.value = "error: " + err;
    }
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
      try {
        await invoke("play_voice", {
          text:   cleaned,
          voice:  selectedVoice.value,
          volume: voiceVolume.value,
          speed:  voiceRate.value,
        });
        lastVoiceError.value = null;
      } catch (err) {
        const msg = String(err);
        console.error("[useVoiceCoach] Erro ao reproduzir:", msg);
        lastVoiceError.value = msg;
      }
    }

    queueProcessing = false;
    isSpeaking.value = false;
  };

  const speak = async (text: string) => {
    if (!voiceEnabled.value) return;
    if (kokoroStatus.value !== "ready") {
      console.warn("[useVoiceCoach] Motor não pronto, ignorando fala.");
      return;
    }
    await stop();
    speakQueue.push(text);
    await processQueue();
  };

  const testVoice = async () => {
    if (isTesting.value) return;
    isTesting.value = true;
    lastVoiceError.value = null;
    try {
      await invoke("play_voice", {
        text:   "Voz do coach ativa. Pronto para a partida!",
        voice:  selectedVoice.value,
        volume: voiceVolume.value,
        speed:  voiceRate.value,
      });
    } catch (err) {
      lastVoiceError.value = String(err);
      console.error("[testVoice]", err);
    } finally {
      isTesting.value = false;
    }
  };

  // ---------------------------------------------------------------------------
  // Inicialização
  // ---------------------------------------------------------------------------
  let _statusRetryInterval: ReturnType<typeof setInterval> | null = null;

  onMounted(() => {
    loadSettings();
    checkKokoroStatus();

    // Retenta a cada 30s até o serviço ficar pronto (HuggingFace Space pode estar dormindo no startup)
    _statusRetryInterval = setInterval(() => {
      if (kokoroStatus.value === 'ready') {
        if (_statusRetryInterval) { clearInterval(_statusRetryInterval); _statusRetryInterval = null; }
        return;
      }
      checkKokoroStatus();
    }, 30_000);
  });

  onUnmounted(() => {
    if (_statusRetryInterval) { clearInterval(_statusRetryInterval); _statusRetryInterval = null; }
  });

  return {
    voiceEnabled,
    voiceVolume,
    voiceRate,
    selectedVoice,
    isSpeaking,
    isTesting,
    kokoroStatus,
    lastVoiceError,
    checkKokoroStatus,
    cleanTextForSpeech,
    speak,
    stop,
    testVoice,
    loadSettings,
    saveSettings,
  };
}
