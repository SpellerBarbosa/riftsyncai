<script setup lang="ts">
import { ref, onMounted } from "vue";
import { WebviewWindow } from "@tauri-apps/api/webviewWindow";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { invoke } from "@tauri-apps/api/core";
import { useVoiceCoach } from "../composables/useVoiceCoach";

import { computed } from "vue";

const voiceCoach = useVoiceCoach();

const voiceEnabled = computed({
  get: () => voiceCoach.voiceEnabled.value,
  set: (val) => { voiceCoach.voiceEnabled.value = val; }
});

const voiceVolume = computed({
  get: () => voiceCoach.voiceVolume.value,
  set: (val) => { voiceCoach.voiceVolume.value = val; }
});

const voiceRate = computed({
  get: () => voiceCoach.voiceRate.value,
  set: (val) => { voiceCoach.voiceRate.value = val; }
});

const selectedVoice = computed({
  get: () => voiceCoach.selectedVoice.value,
  set: (val) => { voiceCoach.selectedVoice.value = val; }
});

const voiceWeights = computed({
  get: () => voiceCoach.voiceWeights.value,
  set: (val) => { voiceCoach.voiceWeights.value = val; }
});

const testVoice = voiceCoach.testVoice;
const saveVoiceSettings = voiceCoach.saveSettings;

const appWindow = getCurrentWindow();
const isClosing = ref(false);

// ── GROQ ────────────────────────────────────────────────────────────────────
const groqKey     = ref("");
const groqModel   = ref("llama-3.1-8b-instant");
const groqShowKey = ref(false);
const groqTesting = ref(false);
const groqSaving  = ref(false);
const groqResult  = ref("");
const groqSuccess = ref<boolean | null>(null);

const groqModels = [
  { id: "llama-3.1-8b-instant",    name: "Llama 3.1 8B Instant (Mais Rápido)" },
  { id: "llama-3.3-70b-versatile", name: "Llama 3.3 70B Versatile (Melhor Qualidade)" },
  { id: "gemma2-9b-it",            name: "Gemma 2 9B IT (Google)" },
  { id: "mixtral-8x7b-32768",      name: "Mixtral 8x7B (Contexto Longo)" },
];

const loadGroqSettings = async () => {
  try {
    const res: any = await invoke("get_groq_settings");
    groqKey.value   = res.api_key;
    groqModel.value = res.model;
  } catch (e) { console.error("Erro ao ler configurações Groq:", e); }
};

const saveGroqSettings = async () => {
  groqSaving.value = true;
  try {
    await invoke("set_groq_settings", {
      enabled: true,          // sempre ativo
      apiKey:  groqKey.value,
      model:   groqModel.value,
    });
  } catch (e) { console.error("Erro ao salvar Groq:", e); }
  finally { setTimeout(() => { groqSaving.value = false; }, 600); }
};

const testGroq = async () => {
  if (!groqKey.value.trim()) {
    groqSuccess.value = false;
    groqResult.value  = "Insira sua Chave de API primeiro.";
    return;
  }
  groqTesting.value = true;
  groqResult.value  = "Testando...";
  groqSuccess.value = null;
  try {
    const res = await invoke<string>("test_groq_connection", {
      apiKey: groqKey.value,
      model:  groqModel.value,
    });
    groqSuccess.value = true;
    groqResult.value  = `Conectado! "${res}"`;
    await saveGroqSettings();
  } catch (e: any) {
    groqSuccess.value = false;
    groqResult.value  = e.toString();
  } finally { groqTesting.value = false; }
};
// ────────────────────────────────────────────────────────────────────────────

const openDataViewer = async () => {
  try {
    let win = await WebviewWindow.getByLabel("data-viewer");
    if (!win) {
      win = new WebviewWindow("data-viewer", {
        url: "index.html",
        title: "Visualizador de Dados",
        width: 800,
        height: 600,
        transparent: true,
        decorations: false,
        center: true,
      });
    }
    await win.show();
    await win.unminimize();
    await win.setFocus();
  } catch (e) {
    console.error("Erro ao abrir visualizador:", e);
  }
};

const minimize = () => appWindow.minimize();
const close = async () => {
  isClosing.value = true;
  saveVoiceSettings();
  await saveGroqSettings();
  setTimeout(() => { appWindow.close(); }, 300);
};

onMounted(() => {
  loadGroqSettings();
});
</script>

<template>
  <div :class="['settings-container', 'glass', { 'fade-out': isClosing }]">
    <header data-tauri-drag-region class="settings-header">
      <div data-tauri-drag-region class="title-drag">
        <h2>Configurações</h2>
      </div>
      <div class="window-controls">
        <button class="control-btn" @click="minimize">_</button>
        <button class="control-btn close" @click="close">×</button>
      </div>
    </header>

    <div class="settings-content scrollable">
      <!-- GROQ IA -->
      <section class="settings-section">
        <div class="section-header">
          <span class="icon">⚡</span>
          <h3>Coach IA — Groq</h3>
        </div>

        <div class="setting-card col">
          <div class="card-info" style="margin-bottom:10px">
            <h4>Inferência Ultrarrápida com Llama 3</h4>
            <p>Sempre ativo. Chave gratuita em <a href="https://console.groq.com/keys" target="_blank">console.groq.com/keys</a>.</p>
          </div>

          <div class="form-group">
            <label class="form-label">Chave de API Groq</label>
            <div class="input-wrapper">
              <input
                :type="groqShowKey ? 'text' : 'password'"
                placeholder="gsk_..."
                v-model="groqKey"
                @change="saveGroqSettings"
                class="form-input"
              />
              <button class="reveal-btn" @click="groqShowKey = !groqShowKey">
                {{ groqShowKey ? '👁️' : '🔒' }}
              </button>
            </div>
          </div>

          <div class="form-group">
            <label class="form-label">Modelo</label>
            <select v-model="groqModel" @change="saveGroqSettings" class="form-select">
              <option v-for="m in groqModels" :key="m.id" :value="m.id">{{ m.name }}</option>
            </select>
          </div>

          <div class="test-row">
            <button class="btn-secondary" :disabled="groqTesting" @click="testGroq">
              {{ groqTesting ? 'Testando...' : 'Testar Conexão' }}
            </button>
            <span v-if="groqSaving && !groqTesting" class="saving-badge">Salvando...</span>
          </div>

          <div v-if="groqResult" :class="['test-feedback', groqSuccess ? 'success' : groqSuccess === false ? 'error' : '']">
            {{ groqResult }}
          </div>
        </div>
      </section>

      <!-- FEEDBACK DE VOZ DO COACH -->
      <section class="settings-section">
        <div class="section-header">
          <span class="icon">🔊</span>
          <h3>Feedback de Voz do Coach</h3>
        </div>

        <div class="setting-card col">
          <div class="switch-row">
            <div class="card-info">
              <h4>Ativar Dicas por Voz</h4>
              <p>O Coach falará as dicas táticas e de itens em tempo real durante a partida.</p>
            </div>
            <label class="switch">
              <input type="checkbox" v-model="voiceEnabled" @change="saveVoiceSettings" />
              <span class="slider"></span>
            </label>
          </div>

          <!-- COLLAPSIBLE VOICE CONFIG SECTION -->
          <div v-if="voiceEnabled" class="ia-collapsible animate-fade">
            <!-- STATUS DO KOKORO LOCAL TTS -->
            <div v-if="voiceCoach.kokoroStatus.value === 'loading'" class="kokoro-loading-box animate-fade">
              <div class="kokoro-progress-text">
                🤖 <strong>Carregando Voz Local Kokoro:</strong> Inicializando ou baixando modelos off-line pela primeira vez (~337MB)...
              </div>
              <div class="kokoro-progress-bar-container">
                <div class="kokoro-progress-bar" style="width: 100%; animation: pulse 2s infinite ease-in-out;"></div>
              </div>
            </div>

            <div v-else-if="voiceCoach.kokoroStatus.value.startsWith('error')" class="kokoro-error-box animate-fade">
              <span class="warning-text">⚠️ Erro ao carregar motor local:</span>
              <p style="font-size: 10px; color: #ff4e4e; margin: 4px 0 0 0;">{{ voiceCoach.kokoroStatus.value }}</p>
            </div>

            <div v-else class="kokoro-loading-box animate-fade" style="border-color: rgba(78, 255, 155, 0.4); background: rgba(78, 255, 155, 0.03);">
              <div class="kokoro-progress-text" style="color: #4eff9b;">
                ⚡ <strong>Kokoro Local ONNX:</strong> Motor de voz off-line 100% pronto e ativo!
              </div>
            </div>

            <div class="form-group">
              <label class="form-label">Selecione a Voz do Coach (Kokoro Local Neural) 🌟</label>
              <select v-model="selectedVoice" @change="saveVoiceSettings" class="form-select" :disabled="voiceCoach.kokoroStatus.value !== 'ready'">
                <option value="francisca">Francisca (Voz Feminina Dora) ✨</option>
                <option value="antonio">Antônio (Voz Masculina Alex) ✨</option>
                <option value="custom">Mistura Personalizada (Dora + Alex) 🎛️</option>
              </select>
              <span class="help-text">
                Vozes neurais brasileiras de alta fidelidade geradas e reproduzidas de forma off-line e super veloz diretamente no seu PC.
              </span>
            </div>

            <!-- PAINEL DE MISTURA MULTI-VOZ PERSONALIZADA -->
            <div v-if="selectedVoice === 'custom'" class="form-group animate-fade">
              <label class="form-label" style="margin-bottom: 8px;">Mesa de Mistura Híbrida (Equalizador de Vozes) 🎛️</label>
              
              <div class="voice-mixer-grid">
                <!-- 🇧🇷 Francisca -->
                <div class="mixer-channel">
                  <div class="mixer-channel-header">
                    <span class="mixer-channel-title">🇧🇷 Francisca (Fem. BR)</span>
                    <span class="mixer-channel-val">{{ voiceWeights.pf_dora }}</span>
                  </div>
                  <input type="range" min="0" max="10" step="1" v-model.number="voiceWeights.pf_dora" @input="saveVoiceSettings" class="form-range" :disabled="voiceCoach.kokoroStatus.value !== 'ready'" />
                </div>

                <!-- 🇧🇷 Antônio -->
                <div class="mixer-channel">
                  <div class="mixer-channel-header">
                    <span class="mixer-channel-title">🇧🇷 Antônio (Masc. BR)</span>
                    <span class="mixer-channel-val">{{ voiceWeights.pm_alex }}</span>
                  </div>
                  <input type="range" min="0" max="10" step="1" v-model.number="voiceWeights.pm_alex" @input="saveVoiceSettings" class="form-range" :disabled="voiceCoach.kokoroStatus.value !== 'ready'" />
                </div>

                <!-- 🇺🇸 Sky -->
                <div class="mixer-channel">
                  <div class="mixer-channel-header">
                    <span class="mixer-channel-title">🇺🇸 Sky (Fem. USA)</span>
                    <span class="mixer-channel-val">{{ voiceWeights.af_sky }}</span>
                  </div>
                  <input type="range" min="0" max="10" step="1" v-model.number="voiceWeights.af_sky" @input="saveVoiceSettings" class="form-range" :disabled="voiceCoach.kokoroStatus.value !== 'ready'" />
                </div>

                <!-- 🇺🇸 Adam -->
                <div class="mixer-channel">
                  <div class="mixer-channel-header">
                    <span class="mixer-channel-title">🇺🇸 Adam (Masc. USA)</span>
                    <span class="mixer-channel-val">{{ voiceWeights.am_adam }}</span>
                  </div>
                  <input type="range" min="0" max="10" step="1" v-model.number="voiceWeights.am_adam" @input="saveVoiceSettings" class="form-range" :disabled="voiceCoach.kokoroStatus.value !== 'ready'" />
                </div>

                <!-- 🇪🇸 Dora -->
                <div class="mixer-channel">
                  <div class="mixer-channel-header">
                    <span class="mixer-channel-title">🇪🇸 Dora (Fem. ESP)</span>
                    <span class="mixer-channel-val">{{ voiceWeights.ef_dora }}</span>
                  </div>
                  <input type="range" min="0" max="10" step="1" v-model.number="voiceWeights.ef_dora" @input="saveVoiceSettings" class="form-range" :disabled="voiceCoach.kokoroStatus.value !== 'ready'" />
                </div>

                <!-- 🇪🇸 Alex -->
                <div class="mixer-channel">
                  <div class="mixer-channel-header">
                    <span class="mixer-channel-title">🇪🇸 Alex (Masc. ESP)</span>
                    <span class="mixer-channel-val">{{ voiceWeights.em_alex }}</span>
                  </div>
                  <input type="range" min="0" max="10" step="1" v-model.number="voiceWeights.em_alex" @input="saveVoiceSettings" class="form-range" :disabled="voiceCoach.kokoroStatus.value !== 'ready'" />
                </div>
              </div>

              <span class="help-text" style="margin-top: 8px;">
                Dica: Ajuste os controles de 0 a 10. O sistema normalizará as proporções automaticamente para criar uma voz híbrida única. Se deixar apenas uma voz ativa, ela falará pura.
              </span>
            </div>

            <div class="form-group">
              <div class="slider-label-row">
                <label class="form-label">Volume ({{ Math.round(voiceVolume * 100) }}%)</label>
              </div>
              <input
                type="range"
                min="0"
                max="1"
                step="0.05"
                v-model.number="voiceVolume"
                @input="saveVoiceSettings"
                class="form-range"
                :disabled="voiceCoach.kokoroStatus.value !== 'ready'"
              />
            </div>

            <div class="form-group">
              <div class="slider-label-row">
                <label class="form-label">Velocidade da Fala ({{ voiceRate.toFixed(1) }}x)</label>
              </div>
              <input
                type="range"
                min="0.5"
                max="2.0"
                step="0.1"
                v-model.number="voiceRate"
                @input="saveVoiceSettings"
                class="form-range"
                :disabled="voiceCoach.kokoroStatus.value !== 'ready'"
              />
            </div>

            <div class="test-row">
              <button class="btn-secondary" @click="testVoice" :disabled="voiceCoach.kokoroStatus.value !== 'ready'">
                🔊 Testar Voz do Coach
              </button>
            </div>
          </div>
        </div>
      </section>

      <!-- ENGINE PERFORMANCE STATUS -->
      <section class="settings-section">
        <div class="section-header">
          <span class="icon">⚡</span>
          <h3>Motor de Performance</h3>
        </div>
        <div class="setting-card col">
          <div class="card-info width-full">
            <div class="promo-header">
              <span class="promo-icon">{{ groqKey.trim() ? '☁️' : '🛡️' }}</span>
              <h5>{{ groqKey.trim() ? 'Modo Assistente Nuvem Ativo' : 'Modo Procedural Determinístico Ativo' }}</h5>
            </div>
            <p class="promo-desc">
              <span v-if="groqKey.trim()">
                O Coach está processando conselhos dinâmicos via cloud LLM. Caso o serviço falte ou a internet caia, o sistema muda automaticamente para o <strong>Modo Offline Procedural</strong> em 0ms, garantindo estabilidade absoluta de FPS.
              </span>
              <span v-else>
                O Spell Coach IA opera no <strong>Modo Super Leve</strong>. Suas dicas e matchups in-game são geradas a partir do banco de dados oficial em <strong>0 milissegundos</strong>, garantindo <strong>0% de uso de CPU/GPU e FPS máximo estável</strong>.
              </span>
            </p>
          </div>
        </div>
      </section>

      <!-- DEV TOOLS -->
      <section class="settings-section">
        <div class="section-header">
          <span class="icon">🛠️</span>
          <h3>Desenvolvedor & Debug</h3>
        </div>
        <div class="setting-card">
          <div class="card-info">
            <h4>Explorador de Dados Estratégicos</h4>
            <p>Acesse matchups, builds detalhadas e gerencie o banco de dados local.</p>
          </div>
          <button class="btn-primary" @click="openDataViewer">
            Abrir Visualizador
          </button>
        </div>
      </section>

      <!-- ABOUT -->
      <section class="settings-section">
        <div class="section-header">
          <span class="icon">ℹ️</span>
          <h3>Sobre o Aplicativo</h3>
        </div>
        <div class="info-card">
          <div class="info-row">
            <span>Versão</span>
            <span class="val">0.1.0 Beta (Nuvem Opcional)</span>
          </div>
          <div class="info-row">
            <span>Desenvolvedor</span>
            <span class="val">Spell Coach IA Team</span>
          </div>
          <div class="info-row">
            <span>Arquitetura</span>
            <span class="val">Híbrida & Alta Performance</span>
          </div>
        </div>
      </section>
    </div>

    <footer class="settings-footer">
      <span>2026 © Spell Coach IA</span>
    </footer>
  </div>
</template>

<style scoped>
.settings-container {
  width: 100vw;
  height: 100vh;
  background: radial-gradient(circle at center, #0a0f19 0%, #05080d 100%);
  color: white;
  padding: 24px;
  display: flex;
  flex-direction: column;
  border: 1px solid var(--accent-gold);
  animation: fadeIn 0.4s ease-out forwards;
  overflow: hidden;
}

.settings-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  border-bottom: 2px solid var(--accent-gold);
  padding-bottom: 12px;
  margin-bottom: 20px;
}

h2 { 
  font-family: 'Beaufort for LOL', serif; 
  color: var(--accent-gold); 
  text-transform: uppercase; 
  letter-spacing: 2px; 
  margin: 0; 
  font-size: 20px;
}

.settings-content {
  flex: 1;
  display: flex;
  flex-direction: column;
  gap: 24px;
  overflow-y: auto;
  padding-right: 6px;
}

/* Custom Scrollbar */
.scrollable::-webkit-scrollbar { width: 4px; }
.scrollable::-webkit-scrollbar-track { background: rgba(0,0,0,0.2); }
.scrollable::-webkit-scrollbar-thumb { background: var(--accent-gold); border-radius: 2px; }

.section-header {
  display: flex;
  align-items: center;
  gap: 12px;
  margin-bottom: 12px;
}

.section-header h3 {
  font-size: 13px;
  text-transform: uppercase;
  color: var(--accent-gold);
  margin: 0;
  letter-spacing: 1px;
}

.setting-card {
  background: rgba(255,255,255,0.03);
  border: 1px solid rgba(255,255,255,0.05);
  border-radius: 8px;
  padding: 16px;
  display: flex;
  flex-direction: column;
}

.setting-card.col {
  align-items: stretch;
  gap: 12px;
}

.switch-row {
  display: flex;
  justify-content: space-between;
  align-items: center;
  width: 100%;
}

.width-full { width: 100%; }

.card-info h4 { font-size: 14px; color: white; margin: 0 0 4px 0; }
.card-info p { font-size: 10px; color: var(--text-secondary); margin: 0; line-height: 1.4; }

/* Form Elements */
.ia-collapsible {
  display: flex;
  flex-direction: column;
  gap: 14px;
  padding-top: 14px;
  border-top: 1px solid rgba(255,255,255,0.05);
}

.form-group {
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.form-label {
  font-size: 10px;
  font-weight: 800;
  text-transform: uppercase;
  color: var(--accent-gold);
  letter-spacing: 0.5px;
}

.input-wrapper {
  display: flex;
  background: rgba(0,0,0,0.4);
  border: 1px solid rgba(200, 170, 110, 0.3);
  border-radius: 4px;
  overflow: hidden;
}

.input-wrapper:focus-within {
  border-color: var(--accent-gold);
}

.form-input {
  flex: 1;
  background: transparent;
  border: none;
  color: white;
  padding: 8px 12px;
  font-size: 12px;
  outline: none;
  font-family: monospace;
}

.reveal-btn {
  background: rgba(255,255,255,0.03);
  border: none;
  border-left: 1px solid rgba(255,255,255,0.05);
  color: white;
  padding: 0 12px;
  cursor: pointer;
}

.reveal-btn:hover { background: rgba(255,255,255,0.08); }

.help-text {
  font-size: 9px;
  color: var(--text-secondary);
}

.help-text a {
  color: var(--accent-blue);
  text-decoration: none;
}

.help-text a:hover {
  text-decoration: underline;
}

.form-select {
  background: rgba(0,0,0,0.4);
  border: 1px solid rgba(200, 170, 110, 0.3);
  border-radius: 4px;
  color: white;
  padding: 8px 12px;
  font-size: 12px;
  outline: none;
  cursor: pointer;
}

.form-select:focus { border-color: var(--accent-gold); }

.test-row {
  display: flex;
  align-items: center;
  gap: 12px;
  margin-top: 6px;
}

.saving-badge {
  font-size: 10px;
  color: #4eff9b;
  opacity: 0.8;
}

.test-feedback {
  font-size: 11px;
  padding: 8px 12px;
  border-radius: 4px;
  background: rgba(255, 255, 255, 0.05);
  border: 1px solid rgba(255,255,255,0.1);
  color: white;
  line-height: 1.4;
  word-break: break-all;
}

.test-feedback.success {
  background: rgba(78, 255, 155, 0.1);
  border-color: #4eff9b;
  color: #4eff9b;
}

.test-feedback.error {
  background: rgba(255, 78, 78, 0.1);
  border-color: #ff4e4e;
  color: #ff4e4e;
}

/* Switch Button */
.switch { position: relative; display: inline-block; width: 34px; height: 18px; flex-shrink: 0; }
.switch input { opacity: 0; width: 0; height: 0; }
.slider { position: absolute; cursor: pointer; top: 0; left: 0; right: 0; bottom: 0; background-color: rgba(255,255,255,0.1); transition: .4s; border-radius: 18px; border: 1px solid rgba(255,255,255,0.2); }
.slider:before { position: absolute; content: ""; height: 12px; width: 12px; left: 2px; bottom: 2px; background-color: white; transition: .4s; border-radius: 50%; }
input:checked + .slider { background-color: var(--accent-gold); border-color: var(--accent-gold); }
input:checked + .slider:before { transform: translateX(16px); }

/* Procedural promo container */
.promo-header {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-bottom: 6px;
}

.promo-icon { font-size: 14px; }
.promo-header h5 { margin: 0; color: var(--accent-blue); font-size: 11px; font-weight: 800; text-transform: uppercase; letter-spacing: 0.5px; }
.promo-desc { font-size: 11px; color: #a0aed0; margin: 0; line-height: 1.5; }

.info-card { background: rgba(0,0,0,0.2); border-radius: 8px; padding: 14px; display: flex; flex-direction: column; gap: 10px; }
.info-row { display: flex; justify-content: space-between; font-size: 11px; }
.info-row span:first-child { color: var(--text-secondary); }
.info-row .val { color: var(--accent-gold); font-weight: 800; }

.settings-footer { margin-top: auto; text-align: center; font-size: 9px; opacity: 0.3; padding-top: 14px; }

.btn-primary, .btn-secondary {
  background: linear-gradient(to bottom, #c89b3c 0%, #785a28 100%);
  border: 1px solid #f0e6d2;
  color: #1e2328;
  padding: 8px 16px;
  font-weight: 800;
  font-size: 11px;
  text-transform: uppercase;
  cursor: pointer;
  transition: all 0.3s ease;
  border-radius: 2px;
}

.btn-secondary {
  background: rgba(255, 255, 255, 0.05);
  border: 1px solid rgba(200, 170, 110, 0.4);
  color: var(--accent-gold);
}

.btn-primary:hover, .btn-secondary:hover {
  filter: brightness(1.2);
  transform: translateY(-1px);
}

.window-controls { display: flex; gap: 4px; }
.control-btn { background: transparent; border: none; color: var(--accent-gold); font-size: 18px; cursor: pointer; padding: 4px 10px; }
.control-btn.close:hover { background: #ff4e4e; color: white; }

.animate-fade {
  animation: fadeIn 0.3s ease-out;
}

@keyframes fadeIn {
  from { opacity: 0; transform: translateY(4px); }
  to { opacity: 1; transform: translateY(0); }
}

/* Custom Hextech Form Range Slider styles */
.form-range {
  -webkit-appearance: none;
  width: 100%;
  height: 6px;
  background: rgba(0, 0, 0, 0.5);
  border: 1px solid rgba(200, 170, 110, 0.2);
  border-radius: 3px;
  outline: none;
  margin: 6px 0;
}

.form-range::-webkit-slider-thumb {
  -webkit-appearance: none;
  appearance: none;
  width: 14px;
  height: 14px;
  border-radius: 50%;
  background: var(--accent-gold);
  border: 1px solid #f0e6d2;
  cursor: pointer;
  transition: transform 0.1s ease;
}

.form-range::-webkit-slider-thumb:hover {
  transform: scale(1.2);
  background: white;
}

.slider-label-row {
  display: flex;
  justify-content: space-between;
  align-items: center;
}

/* Kokoro TTS local neural engine styles */
.kokoro-loading-box {
  background: rgba(0, 0, 0, 0.4);
  border: 1px solid rgba(200, 170, 110, 0.3);
  padding: 12px;
  border-radius: 4px;
  margin-top: 6px;
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.kokoro-progress-text {
  font-size: 11px;
  color: #a0aed0;
}

.kokoro-progress-text strong {
  color: var(--accent-gold);
}

.kokoro-progress-bar-container {
  width: 100%;
  height: 8px;
  background: rgba(0, 0, 0, 0.6);
  border: 1px solid rgba(200, 170, 110, 0.2);
  border-radius: 4px;
  overflow: hidden;
}

.kokoro-progress-bar {
  height: 100%;
  background: linear-gradient(90deg, var(--accent-gold) 0%, #ffffff 100%);
  transition: width 0.3s ease;
  box-shadow: 0 0 8px var(--accent-gold);
}

.kokoro-error-box {
  background: rgba(255, 78, 78, 0.1);
  border: 1px solid #ff4e4e;
  padding: 10px;
  border-radius: 4px;
  margin-top: 6px;
}

.warning-text {
  color: var(--accent-gold) !important;
  font-weight: bold;
}

@keyframes pulse {
  0% { opacity: 0.4; }
  50% { opacity: 0.9; }
  100% { opacity: 0.4; }
}

.voice-mixer-grid {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 10px;
  background: rgba(0, 0, 0, 0.3);
  padding: 10px;
  border-radius: 6px;
  border: 1px solid rgba(200, 170, 110, 0.15);
  margin-top: 4px;
}

.mixer-channel {
  display: flex;
  flex-direction: column;
  gap: 4px;
  background: rgba(255, 255, 255, 0.02);
  padding: 6px 8px;
  border-radius: 4px;
  border: 1px solid rgba(255, 255, 255, 0.03);
}

.mixer-channel:hover {
  background: rgba(255, 255, 255, 0.04);
  border-color: rgba(200, 170, 110, 0.1);
}

.mixer-channel-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.mixer-channel-title {
  font-size: 10px;
  font-weight: 700;
  color: var(--text-secondary);
}

.mixer-channel-val {
  font-size: 9px;
  font-weight: 800;
  color: var(--accent-gold);
  background: rgba(0, 0, 0, 0.4);
  padding: 1px 5px;
  border-radius: 3px;
  border: 1px solid rgba(200, 170, 110, 0.2);
}
</style>
