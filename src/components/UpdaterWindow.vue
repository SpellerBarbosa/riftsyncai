<script setup lang="ts">
import { ref, onMounted, onUnmounted } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { getCurrentWindow } from '@tauri-apps/api/window';

const appWindow = getCurrentWindow();
const version = ref('');
const status = ref<'idle' | 'downloading' | 'animating' | 'error'>('idle');
const errorMsg = ref('');
const downloadPct = ref(0);
const hasTotal = ref(false);
let accumulatedBytes = 0;
let totalBytes = 0;
let unlistenProgress: any = null;

interface ReleaseChange {
  type: 'novo' | 'melhoria' | 'fix';
  text: string;
}
interface Release {
  version: string;
  date: string;
  changes: ReleaseChange[];
}

const releases = ref<Release[]>([]);
const currentRelease = ref<Release | null>(null);
const previousReleases = ref<Release[]>([]);

onMounted(async () => {
  const stored = localStorage.getItem('spellcoach_update_data');
  if (stored) {
    try {
      const parsed = JSON.parse(stored);
      version.value = parsed.version || '';
    } catch (_) {}
  }

  // Carrega changelog estruturado
  try {
    const res = await fetch('/releases.json');
    if (res.ok) {
      const data: Release[] = await res.json();
      releases.value = data;
      // Primeira entrada = versão atual do update; demais = histórico
      if (data.length > 0) {
        currentRelease.value = data[0];
        previousReleases.value = data.slice(1);
      }
    }
  } catch (_) {}

  unlistenProgress = await listen('update-progress', (event: any) => {
    const chunk: number = event.payload.chunk ?? 0;
    const total: number | null = event.payload.total ?? null;
    accumulatedBytes += chunk;
    if (total && total > 0) {
      hasTotal.value = true;
      totalBytes = total;
      downloadPct.value = Math.min(100, Math.round((accumulatedBytes / totalBytes) * 100));
      if (downloadPct.value >= 100 && status.value === 'downloading') {
        triggerNexusExplosion();
      }
    }
  });
});

onUnmounted(() => {
  if (unlistenProgress) unlistenProgress();
});

const closeWindow = () => appWindow.hide();

const installUpdate = async () => {
  status.value = 'downloading';
  accumulatedBytes = 0;
  totalBytes = 0;
  downloadPct.value = 0;
  hasTotal.value = false;
  invoke('download_and_install_update').catch((e: any) => {
    console.error('Update failed:', e);
    errorMsg.value = String(e);
    status.value = 'error';
  });
};

const triggerNexusExplosion = () => { status.value = 'animating'; };

const typeLabel: Record<string, string> = { novo: 'NOVO', melhoria: 'MELHORIA', fix: 'FIX' };
const typeColor: Record<string, string> = {
  novo:     '#4af076',
  melhoria: '#4ab4f0',
  fix:      '#f0a84a',
};
</script>

<template>
  <div class="updater-container">
    <div class="updater-bg"></div>

    <!-- Nexus Animation Overlay -->
    <div v-if="status === 'animating'" class="nexus-explosion-overlay">
      <div class="nexus-crystal"></div>
      <div class="nexus-shockwave"></div>
      <div class="nexus-particles"></div>
      <div class="nexus-flash"></div>
    </div>

    <div class="updater-content" :class="{ 'fade-out': status === 'animating' }">
      <div class="header">
        <div class="logo-box">
          <span class="logo-icon">⚡</span>
        </div>
        <h2 class="title">Atualização Disponível</h2>
        <p class="subtitle">Versão {{ version }}</p>
      </div>

      <div class="changelog-box">
        <div class="changelog-header">
          <span>📝 O que há de novo</span>
          <span v-if="currentRelease" class="changelog-date">{{ currentRelease.date }}</span>
        </div>
        <div class="changelog-content">
          <!-- Versão atual -->
          <template v-if="currentRelease">
            <div
              v-for="(change, i) in currentRelease.changes"
              :key="i"
              class="change-row"
            >
              <span
                class="change-badge"
                :style="{ color: typeColor[change.type], borderColor: typeColor[change.type] }"
              >{{ typeLabel[change.type] }}</span>
              <span class="change-text">{{ change.text }}</span>
            </div>
          </template>
          <div v-else class="no-changes">Pequenas correções e melhorias.</div>

          <!-- Histórico de versões anteriores -->
          <template v-if="previousReleases.length">
            <div class="history-divider">Versões anteriores</div>
            <div v-for="rel in previousReleases" :key="rel.version" class="history-release">
              <div class="history-version">v{{ rel.version }} <span class="history-date">{{ rel.date }}</span></div>
              <div
                v-for="(change, i) in rel.changes"
                :key="i"
                class="change-row change-row--dim"
              >
                <span
                  class="change-badge change-badge--small"
                  :style="{ color: typeColor[change.type], borderColor: typeColor[change.type] }"
                >{{ typeLabel[change.type] }}</span>
                <span class="change-text">{{ change.text }}</span>
              </div>
            </div>
          </template>
        </div>
      </div>

      <div class="actions">
        <template v-if="status === 'idle'">
          <button class="btn btn-secondary" @click="closeWindow">Lembrar Depois</button>
          <button class="btn btn-primary" @click="installUpdate">
            Instalar e Reiniciar
            <span class="btn-glow"></span>
          </button>
        </template>

        <template v-else-if="status === 'downloading'">
          <div class="progress-container">
            <div class="progress-bar-wrapper">
              <!-- Barra determinada (quando servidor informa Content-Length) -->
              <div
                v-if="hasTotal"
                class="progress-bar"
                :style="{ width: downloadPct + '%' }"
              ></div>
              <!-- Barra indeterminada (sem Content-Length) -->
              <div v-else class="progress-bar loading"></div>
            </div>
            <span class="progress-text">
              {{ hasTotal ? `Baixando... ${downloadPct}%` : 'Baixando recursos hextech...' }}
            </span>
          </div>
        </template>

        <template v-else-if="status === 'error'">
          <div class="error-container">
            <span class="error-icon">⚠️</span>
            <span class="error-text">Falha ao atualizar: {{ errorMsg }}</span>
            <button class="btn btn-secondary" style="margin-top:8px" @click="status = 'idle'">Tentar novamente</button>
          </div>
        </template>
      </div>
    </div>
  </div>
</template>

<style scoped>
.updater-container {
  width: 100%;
  height: 100%;
  background: rgba(4, 15, 26, 0.95);
  border: 1px solid #c8aa6e;
  border-radius: 8px;
  overflow: hidden;
  position: relative;
  display: flex;
  flex-direction: column;
  font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, Helvetica, Arial, sans-serif;
  color: #f0e6d2;
  box-shadow: 0 0 30px rgba(0, 0, 0, 0.8), inset 0 0 20px rgba(200, 170, 110, 0.1);
  user-select: none;
}

.updater-bg {
  position: absolute;
  top: 0; left: 0; right: 0; bottom: 0;
  background: radial-gradient(circle at top center, rgba(10, 36, 56, 0.8) 0%, rgba(1, 10, 19, 1) 100%);
  z-index: 0;
}

.updater-content {
  position: relative;
  z-index: 1;
  display: flex;
  flex-direction: column;
  height: 100%;
  padding: 24px;
  transition: opacity 0.5s ease;
}

.updater-content.fade-out {
  opacity: 0;
  pointer-events: none;
}

/* Header */
.header {
  text-align: center;
  margin-bottom: 20px;
}
.logo-box {
  width: 48px;
  height: 48px;
  background: linear-gradient(135deg, #c8aa6e, #7a5c29);
  border-radius: 12px;
  display: flex;
  align-items: center;
  justify-content: center;
  margin: 0 auto 16px;
  box-shadow: 0 4px 15px rgba(200, 170, 110, 0.3);
  position: relative;
}
.logo-box::after {
  content: '';
  position: absolute;
  top: 2px; left: 2px; right: 2px; bottom: 2px;
  border: 1px solid rgba(255, 255, 255, 0.3);
  border-radius: 10px;
}
.logo-icon {
  font-size: 24px;
  filter: drop-shadow(0 2px 4px rgba(0,0,0,0.5));
}
.title {
  font-size: 20px;
  font-weight: 700;
  color: #f0e6d2;
  margin: 0 0 4px;
  letter-spacing: 0.5px;
}
.subtitle {
  font-size: 14px;
  color: #4af0a0;
  margin: 0;
  font-weight: 600;
}

/* Changelog */
.changelog-box {
  flex: 1;
  background: rgba(0, 0, 0, 0.4);
  border: 1px solid rgba(200, 170, 110, 0.3);
  border-radius: 6px;
  display: flex;
  flex-direction: column;
  overflow: hidden;
  margin-bottom: 20px;
}
.changelog-header {
  padding: 8px 12px;
  background: rgba(200, 170, 110, 0.1);
  border-bottom: 1px solid rgba(200, 170, 110, 0.2);
  font-size: 12px;
  font-weight: 700;
  color: #c8aa6e;
  text-transform: uppercase;
  letter-spacing: 1px;
  display: flex;
  justify-content: space-between;
  align-items: center;
}
.changelog-date {
  font-size: 10px;
  color: #5b5a56;
  font-weight: 400;
  text-transform: none;
  letter-spacing: 0;
}
.changelog-content {
  flex: 1;
  padding: 10px 12px;
  overflow-y: auto;
  font-size: 12px;
  line-height: 1.5;
  color: #a09b8c;
  display: flex;
  flex-direction: column;
  gap: 5px;
}

/* Linha de mudança */
.change-row {
  display: flex;
  align-items: flex-start;
  gap: 7px;
}
.change-row--dim { opacity: 0.55; }
.change-badge {
  flex-shrink: 0;
  font-size: 8px;
  font-weight: 700;
  padding: 1px 5px;
  border: 1px solid;
  border-radius: 3px;
  letter-spacing: 0.5px;
  margin-top: 1px;
}
.change-badge--small { font-size: 7px; padding: 1px 4px; }
.change-text { color: #c8b89a; font-size: 11px; line-height: 1.4; }

/* Histórico */
.history-divider {
  margin: 10px 0 6px;
  font-size: 9px;
  text-transform: uppercase;
  letter-spacing: 1px;
  color: #3a3a3a;
  border-top: 1px solid #1a1a1a;
  padding-top: 8px;
}
.history-release { margin-bottom: 8px; }
.history-version {
  font-size: 10px;
  font-weight: 700;
  color: #4a4a4a;
  margin-bottom: 4px;
}
.history-date {
  font-size: 9px;
  font-weight: 400;
  color: #333;
  margin-left: 4px;
}
.no-changes { color: #5b5a56; font-size: 11px; padding: 4px 0; }

/* Scrollbar para o changelog */
.changelog-content::-webkit-scrollbar { width: 6px; }
.changelog-content::-webkit-scrollbar-track { background: rgba(0,0,0,0.2); }
.changelog-content::-webkit-scrollbar-thumb { background: #c8aa6e; border-radius: 3px; }

/* Actions */
.actions {
  display: flex;
  gap: 12px;
  margin-top: auto;
}
.btn {
  flex: 1;
  padding: 12px;
  border: none;
  border-radius: 4px;
  font-size: 14px;
  font-weight: 600;
  cursor: pointer;
  transition: all 0.2s;
  position: relative;
  overflow: hidden;
  text-transform: uppercase;
  letter-spacing: 0.5px;
}
.btn-secondary {
  background: transparent;
  border: 1px solid #5b5a56;
  color: #a09b8c;
}
.btn-secondary:hover {
  border-color: #c8aa6e;
  color: #f0e6d2;
  background: rgba(200, 170, 110, 0.1);
}
.btn-primary {
  background: linear-gradient(180deg, #1e282d 0%, #010a13 100%);
  border: 1px solid #c8aa6e;
  color: #f0e6d2;
  box-shadow: 0 0 10px rgba(200, 170, 110, 0.2);
}
.btn-primary:hover {
  background: linear-gradient(180deg, #2a373d 0%, #0a1929 100%);
  box-shadow: 0 0 15px rgba(200, 170, 110, 0.4);
}
.btn-glow {
  position: absolute;
  top: 0; left: -100%;
  width: 50%; height: 100%;
  background: linear-gradient(90deg, transparent, rgba(255,255,255,0.2), transparent);
  transform: skewX(-20deg);
  animation: btnSweep 3s infinite;
}
@keyframes btnSweep {
  0% { left: -100%; }
  20% { left: 200%; }
  100% { left: 200%; }
}

/* Progress */
.progress-container {
  width: 100%;
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 10px;
}
.progress-bar-wrapper {
  width: 100%;
  height: 6px;
  background: rgba(0, 0, 0, 0.5);
  border: 1px solid rgba(200, 170, 110, 0.3);
  border-radius: 3px;
  overflow: hidden;
  position: relative;
}
.progress-bar {
  height: 100%;
  width: 0%;
  background: linear-gradient(90deg, #c8aa6e, #f0e84a);
  transition: width 0.2s linear;
}
.progress-bar.loading {
  width: 100%;
  background: linear-gradient(90deg, transparent, #c8aa6e, transparent);
  background-size: 200% 100%;
  animation: indeterminate 1.5s infinite linear;
}
@keyframes indeterminate {
  0% { background-position: 200% 0; }
  100% { background-position: -200% 0; }
}
.progress-text {
  font-size: 12px;
  color: #a09b8c;
  animation: pulseText 1.5s infinite;
}
@keyframes pulseText {
  0% { opacity: 0.6; }
  50% { opacity: 1; }
  100% { opacity: 0.6; }
}

/* Erro */
.error-container {
  width: 100%;
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 6px;
  padding: 8px 0;
}
.error-icon { font-size: 20px; }
.error-text {
  font-size: 11px;
  color: #ff6b6b;
  text-align: center;
  word-break: break-word;
}

/* =========================================================================
   ANIMAÇÃO DE EXPLOSÃO DO NEXUS
   ========================================================================= */
.nexus-explosion-overlay {
  position: absolute;
  top: 0; left: 0; right: 0; bottom: 0;
  z-index: 10;
  display: flex;
  align-items: center;
  justify-content: center;
  pointer-events: none;
}

.nexus-crystal {
  width: 40px;
  height: 80px;
  background: linear-gradient(135deg, #4ab4f0, #0a4682);
  clip-path: polygon(50% 0%, 100% 25%, 100% 75%, 50% 100%, 0% 75%, 0% 25%);
  box-shadow: 0 0 30px #4ab4f0;
  animation: nexusFloat 1s ease-in-out infinite alternate, nexusShatter 2.5s forwards;
}

.nexus-shockwave {
  position: absolute;
  width: 10px;
  height: 10px;
  border-radius: 50%;
  border: 4px solid #4ab4f0;
  opacity: 0;
  animation: shockwave 2.5s forwards;
}

.nexus-flash {
  position: absolute;
  top: 0; left: 0; right: 0; bottom: 0;
  background: white;
  opacity: 0;
  animation: finalFlash 2.5s forwards;
}

@keyframes nexusFloat {
  0% { transform: translateY(-5px); }
  100% { transform: translateY(5px); }
}

@keyframes nexusShatter {
  0% { transform: scale(1) rotate(0deg); filter: brightness(1); opacity: 1; }
  40% { transform: scale(1.2) rotate(5deg); filter: brightness(2); opacity: 1; }
  45% { transform: scale(1.3) rotate(-5deg); filter: brightness(3); opacity: 1; }
  50% { transform: scale(0) rotate(20deg); opacity: 0; }
  100% { transform: scale(0); opacity: 0; }
}

@keyframes shockwave {
  0% { transform: scale(1); opacity: 0; }
  49% { transform: scale(1); opacity: 0; }
  50% { transform: scale(1); opacity: 1; border-width: 20px; }
  70% { transform: scale(40); opacity: 0; border-width: 1px; }
  100% { transform: scale(40); opacity: 0; }
}

@keyframes finalFlash {
  0% { opacity: 0; }
  50% { opacity: 0; }
  55% { opacity: 1; }
  80% { opacity: 1; }
  100% { opacity: 0; }
}
</style>
