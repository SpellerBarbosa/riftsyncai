<script setup lang="ts">
import { ref, onMounted } from "vue";
import { WebviewWindow } from "@tauri-apps/api/webviewWindow";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { invoke } from "@tauri-apps/api/core";
import { useVoiceCoach, VOICE_OPTIONS } from "../composables/useVoiceCoach";

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

const testVoice = voiceCoach.testVoice;
const saveVoiceSettings = voiceCoach.saveSettings;

const audioTestResult = ref<string | null>(null);
const audioTestLoading = ref(false);
const audioDeviceStatus = ref<string>("");

const loadAudioStatus = async () => {
  try {
    audioDeviceStatus.value = await invoke<string>("get_audio_status");
  } catch { /* silencioso */ }
};

const testAudioDevice = async () => {
  audioTestResult.value = null;
  audioTestLoading.value = true;
  try {
    const result = await invoke<string>("test_audio_output");
    audioTestResult.value = "✅ " + result;
    await loadAudioStatus();
  } catch (err: any) {
    audioTestResult.value = "❌ " + String(err);
    await loadAudioStatus();
  } finally {
    audioTestLoading.value = false;
  }
};

const appWindow = getCurrentWindow();
const isClosing = ref(false);

// ── GROQ (chave oculta — usada só para indicador de modo no promo box) ───────
const groqKey = ref("");
const loadGroqSettings = async () => {
  try {
    const res: any = await invoke("get_groq_settings");
    groqKey.value = res.api_key;
  } catch { /* silencioso */ }
};
// ────────────────────────────────────────────────────────────────────────────

const forceSyncing = ref(false);
const forceSyncMsg = ref('');
const forceSyncOk  = ref<boolean | null>(null);

const forceSync = async () => {
  forceSyncing.value = true;
  forceSyncMsg.value = 'Sincronizando...';
  forceSyncOk.value  = null;
  try {
    await invoke('force_sync_vercel_command');
    forceSyncMsg.value = 'Sincronização concluída!';
    forceSyncOk.value  = true;
  } catch (e: any) {
    forceSyncMsg.value = `Erro: ${e}`;
    forceSyncOk.value  = false;
  } finally {
    forceSyncing.value = false;
  }
};

const coverageResult = ref<any>(null);
const coverageLoading = ref(false);

const checkCoverage = async () => {
  coverageLoading.value = true;
  try {
    coverageResult.value = await invoke('get_sync_coverage');
  } catch (e: any) {
    coverageResult.value = { error: String(e) };
  } finally {
    coverageLoading.value = false;
  }
};

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
  await loadGroqSettings();
  setTimeout(() => { appWindow.close(); }, 300);
};

onMounted(() => {
  loadGroqSettings();
  loadAudioStatus();
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

          <div v-if="voiceEnabled" class="ia-collapsible animate-fade">

            <!-- STATUS DA API -->
            <div
              class="kokoro-loading-box animate-fade"
              :style="voiceCoach.kokoroStatus.value === 'ready'
                ? 'border-color: rgba(78,255,155,0.4); background: rgba(78,255,155,0.03);'
                : ''"
            >
              <div
                class="kokoro-progress-text"
                :style="voiceCoach.kokoroStatus.value === 'ready' ? 'color:#4eff9b;' : ''"
              >
                <template v-if="voiceCoach.kokoroStatus.value === 'loading'">
                  🤖 <strong>API de Voz:</strong> Conectando...
                </template>
                <template v-else-if="voiceCoach.kokoroStatus.value.startsWith('error')">
                  ⚠️ <strong>Erro na API de voz:</strong> {{ voiceCoach.kokoroStatus.value }}
                </template>
                <template v-else>
                  ⚡ <strong>API de Voz:</strong> Pronta!
                  <span v-if="audioDeviceStatus && audioDeviceStatus !== 'ok'" style="margin-left:8px; font-size:10px; color:#ff9f43;">
                    ⚠️ Áudio: {{ audioDeviceStatus }}
                  </span>
                </template>
              </div>
            </div>

            <!-- SELEÇÃO DE VOZ -->
            <div class="form-group">
              <label class="form-label">Voz do Coach</label>
              <select v-model="selectedVoice" @change="saveVoiceSettings" class="form-select">
                <option v-for="opt in VOICE_OPTIONS" :key="opt.value" :value="opt.value">
                  {{ opt.label }}
                </option>
              </select>
              <span class="help-text">Vozes neurais geradas via API — qualidade igual ao Kokoro local.</span>
            </div>

            <!-- VOLUME -->
            <div class="form-group">
              <div class="slider-label-row">
                <label class="form-label">Volume ({{ Math.round(voiceVolume * 100) }}%)</label>
              </div>
              <input type="range" min="0" max="1" step="0.05"
                v-model.number="voiceVolume" @input="saveVoiceSettings" class="form-range" />
            </div>

            <!-- VELOCIDADE -->
            <div class="form-group">
              <div class="slider-label-row">
                <label class="form-label">Velocidade da Fala ({{ voiceRate.toFixed(1) }}x)</label>
              </div>
              <input type="range" min="0.5" max="2.0" step="0.1"
                v-model.number="voiceRate" @input="saveVoiceSettings" class="form-range" />
            </div>

            <!-- BOTÕES DE TESTE -->
            <div class="test-row">
              <button class="btn-secondary" @click="testVoice" :disabled="voiceCoach.isTesting.value">
                {{ voiceCoach.isTesting.value ? "⏳ Sintetizando..." : "🔊 Testar Voz" }}
              </button>
              <div v-if="voiceCoach.isTesting.value" style="margin-top:4px; font-size:10px; color:#aaa;">
                Primeira síntese pode levar até 60s enquanto o servidor acorda.
              </div>
              <div v-if="voiceCoach.lastVoiceError.value" style="margin-top:6px; font-size:10px; color:#ff4e4e; background:rgba(255,78,78,0.08); border:1px solid rgba(255,78,78,0.3); padding:6px 8px; border-radius:4px; word-break:break-all;">
                ❌ {{ voiceCoach.lastVoiceError.value }}
              </div>
              <button class="btn-secondary" @click="testAudioDevice" :disabled="audioTestLoading" style="margin-top:6px; opacity:0.8;">
                🔧 Diagnosticar Saída de Áudio
              </button>
              <div v-if="audioTestResult" style="margin-top:6px; font-size:10px; padding:6px 8px; border-radius:6px; background:rgba(255,255,255,0.05); word-break:break-word;">
                {{ audioTestResult }}
              </div>
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

        <div class="setting-card">
          <div class="card-info">
            <h4>Forçar Sincronização</h4>
            <p>Ignora o cache de 2h e baixa os dados mais recentes da API agora.</p>
            <span v-if="forceSyncMsg" :class="forceSyncOk === true ? 'sync-ok' : forceSyncOk === false ? 'sync-err' : 'sync-info'">
              {{ forceSyncMsg }}
            </span>
          </div>
          <button class="btn-secondary" :disabled="forceSyncing" @click="forceSync">
            {{ forceSyncing ? '⏳ Sincronizando...' : '🔄 Forçar Sync' }}
          </button>
        </div>

        <div class="setting-card">
          <div class="card-info">
            <h4>Cobertura do Banco</h4>
            <p>Verifica quais ELOs foram populados com sucesso após o sync.</p>
            <div v-if="coverageResult && !coverageResult.error" class="coverage-grid">
              <div class="coverage-row">
                <span>Campeões</span>
                <strong>{{ coverageResult.champions_with_data }}</strong>
              </div>
              <div class="coverage-row">
                <span>Wards (Challenger)</span>
                <strong>{{ coverageResult.wards_challenger }}</strong>
              </div>
              <div class="coverage-section">Tier List por ELO:</div>
              <div v-for="(count, elo) in coverageResult.tier_list_por_elo" :key="elo" class="coverage-row">
                <span>{{ elo }}</span>
                <strong :class="count > 0 ? 'sync-ok' : 'sync-err'">{{ count }} entradas</strong>
              </div>
            </div>
            <span v-if="coverageResult?.error" class="sync-err">{{ coverageResult.error }}</span>
          </div>
          <button class="btn-secondary" :disabled="coverageLoading" @click="checkCoverage">
            {{ coverageLoading ? '⏳ Verificando...' : '🔍 Verificar Cobertura' }}
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

.sync-ok   { font-size: 9px; color: #4af0a0; margin-top: 4px; display: block; }
.sync-err  { font-size: 9px; color: #f04a4a; margin-top: 4px; display: block; }
.sync-info { font-size: 9px; color: #a09b8c; margin-top: 4px; display: block; }

.coverage-grid { margin-top: 6px; display: flex; flex-direction: column; gap: 2px; }
.coverage-row  { display: flex; justify-content: space-between; font-size: 9px; color: #a09b8c; }
.coverage-row strong { font-size: 9px; }
.coverage-section { font-size: 8px; color: #5a5a5a; margin-top: 4px; text-transform: uppercase; letter-spacing: 0.5px; }

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
