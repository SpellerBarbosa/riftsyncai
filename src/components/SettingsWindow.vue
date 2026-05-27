<script setup lang="ts">
import { ref, computed, onMounted } from "vue";
import { WebviewWindow } from "@tauri-apps/api/webviewWindow";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { invoke } from "@tauri-apps/api/core";
import { getVersion } from "@tauri-apps/api/app";
import { useVoiceCoach, VOICE_OPTIONS } from "../composables/useVoiceCoach";
import { useCoachSettings } from "../composables/useCoachSettings";

const voiceCoach = useVoiceCoach();
const appVersion = ref("...");
onMounted(async () => { try { appVersion.value = await getVersion(); } catch { appVersion.value = "0.2.20"; } });

const voiceEnabled  = computed({ get: () => voiceCoach.voiceEnabled.value,  set: v => { voiceCoach.voiceEnabled.value = v; } });
const voiceVolume   = computed({ get: () => voiceCoach.voiceVolume.value,   set: v => { voiceCoach.voiceVolume.value = v; } });
const voiceRate     = computed({ get: () => voiceCoach.voiceRate.value,     set: v => { voiceCoach.voiceRate.value = v; } });
const selectedVoice = computed({ get: () => voiceCoach.selectedVoice.value, set: v => { voiceCoach.selectedVoice.value = v; } });
const testVoice = voiceCoach.testVoice;
const saveVoiceSettings = voiceCoach.saveSettings;

const coachSettings = useCoachSettings();
const wardAlertsEnabled = computed({ get: () => coachSettings.wardAlertsEnabled.value, set: v => { coachSettings.wardAlertsEnabled.value = v; coachSettings.saveSettings(); } });
const skillTipsEnabled  = computed({ get: () => coachSettings.skillTipsEnabled.value,  set: v => { coachSettings.skillTipsEnabled.value = v;  coachSettings.saveSettings(); } });

const audioTestResult  = ref<string | null>(null);
const audioTestLoading = ref(false);
const audioDeviceStatus = ref("");

const loadAudioStatus = async () => {
  try { audioDeviceStatus.value = await invoke<string>("get_audio_status"); } catch { /* silent */ }
};

const testAudioDevice = async () => {
  audioTestResult.value = null; audioTestLoading.value = true;
  try {
    const result = await invoke<string>("test_audio_output");
    audioTestResult.value = "✅ " + result;
    await loadAudioStatus();
  } catch (err: any) {
    audioTestResult.value = "❌ " + String(err);
    await loadAudioStatus();
  } finally { audioTestLoading.value = false; }
};

const appWindow = getCurrentWindow();
const isClosing = ref(false);

const groqKey = ref("");
const loadGroqSettings = async () => {
  try { const res: any = await invoke("get_groq_settings"); groqKey.value = res.api_key; } catch { /* silent */ }
};

const forceSyncing  = ref(false);
const forceSyncMsg  = ref('');
const forceSyncOk   = ref<boolean | null>(null);

const forceSync = async () => {
  forceSyncing.value = true; forceSyncMsg.value = 'Sincronizando...'; forceSyncOk.value = null;
  try { await invoke('force_sync_vercel_command'); forceSyncMsg.value = 'Sincronização concluída!'; forceSyncOk.value = true; }
  catch (e: any) { forceSyncMsg.value = `Erro: ${e}`; forceSyncOk.value = false; }
  finally { forceSyncing.value = false; }
};

const coverageResult  = ref<any>(null);
const coverageLoading = ref(false);
const checkCoverage = async () => {
  coverageLoading.value = true;
  try { coverageResult.value = await invoke('get_sync_coverage'); }
  catch (e: any) { coverageResult.value = { error: String(e) }; }
  finally { coverageLoading.value = false; }
};

const openDataViewer = async () => {
  try {
    let win = await WebviewWindow.getByLabel("data-viewer");
    if (!win) {
      win = new WebviewWindow("data-viewer", { url: "index.html", title: "Visualizador de Dados", width: 800, height: 600, transparent: true, decorations: false, center: true });
    }
    await win.show(); await win.unminimize(); await win.setFocus();
  } catch (e) { console.error("Erro ao abrir visualizador:", e); }
};

const minimize = () => appWindow.minimize();
const close = async () => {
  isClosing.value = true; saveVoiceSettings();
  await loadGroqSettings();
  setTimeout(() => appWindow.close(), 300);
};

onMounted(() => { loadGroqSettings(); loadAudioStatus(); });
</script>

<template>
  <div class="w-screen h-screen flex flex-col border border-[#c8aa6e] text-white overflow-hidden transition-all duration-300"
       :class="isClosing ? 'opacity-0 scale-95' : 'animate-[fadeIn_0.4s_ease-out_forwards]'"
       style="background:radial-gradient(circle at center,#0a0f19 0%,#05080d 100%)">

    <!-- Header -->
    <header class="flex justify-between items-center border-b-2 border-[#c8aa6e] pb-3 mb-5 px-6 pt-6 shrink-0" data-tauri-drag-region>
      <div data-tauri-drag-region>
        <h2 class="font-bold text-[#c8aa6e] uppercase tracking-[2px] m-0 text-xl">Configurações</h2>
      </div>
      <div class="flex gap-1">
        <button class="bg-transparent border-none text-[#c8aa6e] text-lg cursor-pointer px-2.5 py-1" @click="minimize">_</button>
        <button class="bg-transparent border-none text-[#c8aa6e] text-lg cursor-pointer px-2.5 py-1 hover:bg-[#ff4e4e] hover:text-white transition-colors" @click="close">×</button>
      </div>
    </header>

    <!-- Content -->
    <div class="flex-1 flex flex-col gap-6 overflow-y-auto pr-1.5 px-6 pb-4 [&::-webkit-scrollbar]:w-1 [&::-webkit-scrollbar-thumb]:bg-[#c8aa6e] [&::-webkit-scrollbar-track]:bg-black/20">

      <!-- Voice -->
      <section>
        <div class="flex items-center gap-3 mb-3">
          <span>🔊</span>
          <h3 class="text-[13px] uppercase text-[#c8aa6e] m-0 tracking-wider">Feedback de Voz do Coach</h3>
        </div>
        <div class="bg-white/3 border border-white/5 rounded-lg p-4 flex flex-col gap-3">
          <div class="flex justify-between items-center w-full">
            <div>
              <h4 class="text-sm text-white m-0 mb-1">Ativar Dicas por Voz</h4>
              <p class="text-[10px] text-[#70728a] m-0 leading-tight">O Coach falará as dicas táticas e de itens em tempo real durante a partida.</p>
            </div>
            <label class="relative inline-block w-8.5 h-4.5 shrink-0">
              <input type="checkbox" class="opacity-0 w-0 h-0" v-model="voiceEnabled" @change="saveVoiceSettings" />
              <span class="switch-slider absolute cursor-pointer inset-0 rounded-[18px] border border-white/20 transition-[0.4s]" :class="voiceEnabled ? 'bg-[#c8aa6e] border-[#c8aa6e]' : 'bg-white/10'"></span>
            </label>
          </div>

          <div v-if="voiceEnabled" class="flex flex-col gap-3 pt-3 border-t border-white/5 animate-[fadeIn_0.3s_ease-out]">
            <!-- API status -->
            <div class="bg-black/40 border rounded p-3"
                 :class="voiceCoach.kokoroStatus.value === 'ready' ? 'border-[rgba(78,255,155,0.4)]' : 'border-[rgba(200,170,110,0.3)]'">
              <p class="text-[11px] text-[#a0aed0] m-0"
                 :class="voiceCoach.kokoroStatus.value === 'ready' ? '!text-[#4eff9b]' : ''">
                <template v-if="voiceCoach.kokoroStatus.value === 'loading'">🤖 <strong>API de Voz:</strong> Conectando...</template>
                <template v-else-if="voiceCoach.kokoroStatus.value.startsWith('error')">⚠️ <strong>Erro:</strong> {{ voiceCoach.kokoroStatus.value }}</template>
                <template v-else>⚡ <strong class="text-[#c8aa6e]">API de Voz:</strong> Pronta!
                  <span v-if="audioDeviceStatus && audioDeviceStatus !== 'ok'" class="ml-2 text-[10px] text-[#ff9f43]">⚠️ Áudio: {{ audioDeviceStatus }}</span>
                </template>
              </p>
            </div>

            <!-- Voice select -->
            <div class="flex flex-col gap-1.5">
              <label class="text-[10px] font-extrabold uppercase text-[#c8aa6e] tracking-wide">Voz do Coach</label>
              <select v-model="selectedVoice" @change="saveVoiceSettings"
                      class="bg-black/40 border border-[rgba(200,170,110,0.3)] rounded text-white px-3 py-2 text-[12px] outline-none cursor-pointer focus:border-[#c8aa6e]">
                <option v-for="opt in VOICE_OPTIONS" :key="opt.value" :value="opt.value">{{ opt.label }}</option>
              </select>
              <span class="text-[9px] text-[#70728a]">Vozes neurais geradas via API — qualidade igual ao Kokoro local.</span>
            </div>

            <!-- Volume -->
            <div class="flex flex-col gap-1.5">
              <label class="text-[10px] font-extrabold uppercase text-[#c8aa6e] tracking-wide">Volume ({{ Math.round(voiceVolume * 100) }}%)</label>
              <input type="range" min="0" max="1" step="0.05" v-model.number="voiceVolume" @input="saveVoiceSettings" class="range-slider" />
            </div>

            <!-- Rate -->
            <div class="flex flex-col gap-1.5">
              <label class="text-[10px] font-extrabold uppercase text-[#c8aa6e] tracking-wide">Velocidade da Fala ({{ voiceRate.toFixed(1) }}x)</label>
              <input type="range" min="0.5" max="2.0" step="0.1" v-model.number="voiceRate" @input="saveVoiceSettings" class="range-slider" />
            </div>

            <!-- Test buttons -->
            <div class="flex flex-col gap-1.5 mt-1.5">
              <button class="btn-secondary" @click="testVoice" :disabled="voiceCoach.isTesting.value">
                {{ voiceCoach.isTesting.value ? "⏳ Sintetizando..." : "🔊 Testar Voz" }}
              </button>
              <div v-if="voiceCoach.isTesting.value" class="text-[10px] text-[#aaa]">Primeira síntese pode levar até 60s.</div>
              <div v-if="voiceCoach.lastVoiceError.value" class="text-[10px] text-[#ff4e4e] bg-[rgba(255,78,78,0.08)] border border-[rgba(255,78,78,0.3)] p-2 rounded break-all">
                ❌ {{ voiceCoach.lastVoiceError.value }}
              </div>
              <button class="btn-secondary opacity-80" @click="testAudioDevice" :disabled="audioTestLoading">🔧 Diagnosticar Saída de Áudio</button>
              <div v-if="audioTestResult" class="text-[10px] p-2 rounded bg-white/5 break-words">{{ audioTestResult }}</div>
            </div>
          </div>
        </div>
      </section>

      <!-- Game alerts -->
      <section>
        <div class="flex items-center gap-3 mb-3">
          <span>👁️</span>
          <h3 class="text-[13px] uppercase text-[#c8aa6e] m-0 tracking-wider">Alertas de Jogo</h3>
        </div>
        <div class="bg-white/3 border border-white/5 rounded-lg p-4 flex flex-col gap-3">
          <div class="flex justify-between items-center w-full">
            <div>
              <h4 class="text-sm text-white m-0 mb-1">Mapa de Wards Geral</h4>
              <p class="text-[10px] text-[#70728a] m-0 leading-tight">Exibe sugestões de ward a cada 2 minutos e após abates.</p>
            </div>
            <label class="relative inline-block w-8.5 h-4.5 shrink-0">
              <input type="checkbox" class="opacity-0 w-0 h-0" v-model="wardAlertsEnabled" />
              <span class="switch-slider absolute cursor-pointer inset-0 rounded-[18px] border border-white/20 transition-[0.4s]" :class="wardAlertsEnabled ? 'bg-[#c8aa6e] border-[#c8aa6e]' : 'bg-white/10'"></span>
            </label>
          </div>
          <div class="flex justify-between items-center w-full">
            <div>
              <h4 class="text-sm text-white m-0 mb-1">Dicas de Habilidades</h4>
              <p class="text-[10px] text-[#70728a] m-0 leading-tight">Exibe qual skill evoluir ao subir de nível.</p>
            </div>
            <label class="relative inline-block w-8.5 h-4.5 shrink-0">
              <input type="checkbox" class="opacity-0 w-0 h-0" v-model="skillTipsEnabled" />
              <span class="switch-slider absolute cursor-pointer inset-0 rounded-[18px] border border-white/20 transition-[0.4s]" :class="skillTipsEnabled ? 'bg-[#c8aa6e] border-[#c8aa6e]' : 'bg-white/10'"></span>
            </label>
          </div>
        </div>
      </section>

      <!-- Engine -->
      <section>
        <div class="flex items-center gap-3 mb-3">
          <span>⚡</span>
          <h3 class="text-[13px] uppercase text-[#c8aa6e] m-0 tracking-wider">Motor de Performance</h3>
        </div>
        <div class="bg-white/3 border border-white/5 rounded-lg p-4">
          <div class="flex items-center gap-2 mb-1.5">
            <span class="text-sm">{{ groqKey.trim() ? '☁️' : '🛡️' }}</span>
            <h5 class="m-0 text-[#4ab4f0] text-[11px] font-extrabold uppercase tracking-wide">{{ groqKey.trim() ? 'Modo Assistente Nuvem Ativo' : 'Modo Procedural Determinístico Ativo' }}</h5>
          </div>
          <p class="text-[11px] text-[#a0aed0] m-0 leading-relaxed">
            <span v-if="groqKey.trim()">O Coach está processando conselhos dinâmicos via cloud LLM. Caso o serviço falte, o sistema muda automaticamente para o <strong>Modo Offline Procedural</strong> em 0ms.</span>
            <span v-else>Opera no <strong>Modo Super Leve</strong>. Dicas geradas a partir do banco de dados em <strong>0ms</strong>, garantindo <strong>0% de uso de CPU/GPU</strong>.</span>
          </p>
        </div>
      </section>

      <!-- Dev tools -->
      <section>
        <div class="flex items-center gap-3 mb-3">
          <span>🛠️</span>
          <h3 class="text-[13px] uppercase text-[#c8aa6e] m-0 tracking-wider">Desenvolvedor &amp; Debug</h3>
        </div>
        <div class="flex flex-col gap-3">
          <div class="bg-white/3 border border-white/5 rounded-lg p-4 flex justify-between items-center">
            <div>
              <h4 class="text-sm text-white m-0 mb-1">Explorador de Dados Estratégicos</h4>
              <p class="text-[10px] text-[#70728a] m-0">Acesse matchups, builds e o banco de dados local.</p>
            </div>
            <button class="btn-primary" @click="openDataViewer">Abrir Visualizador</button>
          </div>

          <div class="bg-white/3 border border-white/5 rounded-lg p-4 flex justify-between items-start">
            <div>
              <h4 class="text-sm text-white m-0 mb-1">Forçar Sincronização</h4>
              <p class="text-[10px] text-[#70728a] m-0 mb-1">Ignora o cache de 2h e baixa dados mais recentes agora.</p>
              <span v-if="forceSyncMsg" class="text-[9px] block mt-1"
                    :class="forceSyncOk === true ? 'text-[#4af0a0]' : forceSyncOk === false ? 'text-[#f04a4a]' : 'text-[#a09b8c]'">
                {{ forceSyncMsg }}
              </span>
            </div>
            <button class="btn-secondary" :disabled="forceSyncing" @click="forceSync">
              {{ forceSyncing ? '⏳ Sincronizando...' : '🔄 Forçar Sync' }}
            </button>
          </div>

          <div class="bg-white/3 border border-white/5 rounded-lg p-4 flex justify-between items-start">
            <div class="flex-1 min-w-0 mr-3">
              <h4 class="text-sm text-white m-0 mb-1">Cobertura do Banco</h4>
              <p class="text-[10px] text-[#70728a] m-0 mb-1">Verifica quais ELOs foram populados com sucesso.</p>
              <div v-if="coverageResult && !coverageResult.error" class="flex flex-col gap-0.5 mt-1.5">
                <div class="flex justify-between text-[9px] text-[#a09b8c]"><span>Campeões</span><strong>{{ coverageResult.champions_with_data }}</strong></div>
                <div class="flex justify-between text-[9px] text-[#a09b8c]"><span>Wards (Challenger)</span><strong>{{ coverageResult.wards_challenger }}</strong></div>
                <div class="text-[8px] text-[#5a5a5a] mt-1 uppercase tracking-wide">Tier List por ELO:</div>
                <div v-for="(count, elo) in coverageResult.tier_list_por_elo" :key="elo" class="flex justify-between text-[9px] text-[#a09b8c]">
                  <span>{{ elo }}</span>
                  <strong :class="count > 0 ? 'text-[#4af0a0]' : 'text-[#f04a4a]'">{{ count }} entradas</strong>
                </div>
              </div>
              <span v-if="coverageResult?.error" class="text-[9px] text-[#f04a4a] mt-1 block">{{ coverageResult.error }}</span>
            </div>
            <button class="btn-secondary shrink-0" :disabled="coverageLoading" @click="checkCoverage">
              {{ coverageLoading ? '⏳ Verificando...' : '🔍 Verificar Cobertura' }}
            </button>
          </div>
        </div>
      </section>

      <!-- About -->
      <section>
        <div class="flex items-center gap-3 mb-3">
          <span>ℹ️</span>
          <h3 class="text-[13px] uppercase text-[#c8aa6e] m-0 tracking-wider">Sobre o Aplicativo</h3>
        </div>
        <div class="bg-black/20 rounded-lg p-3.5 flex flex-col gap-2.5">
          <div class="flex justify-between text-[11px]"><span class="text-[#70728a]">Versão</span><span class="text-[#c8aa6e] font-extrabold">{{ appVersion }}</span></div>
          <div class="flex justify-between text-[11px]"><span class="text-[#70728a]">Desenvolvedor</span><span class="text-[#c8aa6e] font-extrabold">Spell Coach IA Team</span></div>
          <div class="flex justify-between text-[11px]"><span class="text-[#70728a]">Arquitetura</span><span class="text-[#c8aa6e] font-extrabold">Híbrida &amp; Alta Performance</span></div>
        </div>
      </section>
    </div>

    <footer class="mt-auto text-center text-[9px] opacity-30 py-3.5 px-6 shrink-0">2026 © Spell Coach IA</footer>
  </div>
</template>

<style scoped>
/* Switch thumb */
.switch-slider::before {
  position: absolute; content: ""; height: 12px; width: 12px;
  left: 2px; bottom: 2px; background: white;
  transition: .4s; border-radius: 50%;
}
input:checked + .switch-slider::before { transform: translateX(16px); }

/* Range slider */
.range-slider {
  -webkit-appearance: none; width: 100%; height: 6px;
  background: rgba(0,0,0,0.5); border: 1px solid rgba(200,170,110,0.2);
  border-radius: 3px; outline: none; margin: 6px 0;
}
.range-slider::-webkit-slider-thumb {
  -webkit-appearance: none; width: 14px; height: 14px; border-radius: 50%;
  background: #c8aa6e; border: 1px solid #f0e6d2; cursor: pointer; transition: transform .1s;
}
.range-slider::-webkit-slider-thumb:hover { transform: scale(1.2); background: white; }

/* Buttons */
.btn-primary {
  background: linear-gradient(to bottom, #c89b3c 0%, #785a28 100%);
  border: 1px solid #f0e6d2; color: #1e2328;
  padding: 8px 16px; font-weight: 800; font-size: 11px;
  text-transform: uppercase; cursor: pointer; transition: all .3s; border-radius: 2px;
}
.btn-secondary {
  background: rgba(255,255,255,0.05); border: 1px solid rgba(200,170,110,0.4);
  color: #c8aa6e; padding: 8px 16px; font-weight: 800; font-size: 11px;
  text-transform: uppercase; cursor: pointer; transition: all .3s; border-radius: 2px;
}
.btn-primary:hover, .btn-secondary:hover { filter: brightness(1.2); transform: translateY(-1px); }
.btn-primary:disabled, .btn-secondary:disabled { opacity: 0.5; cursor: not-allowed; transform: none; }

@keyframes fadeIn { from { opacity:0; transform:translateY(4px); } to { opacity:1; transform:translateY(0); } }
</style>
