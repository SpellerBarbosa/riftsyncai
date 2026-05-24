<script setup lang="ts">
import { ref, onMounted, onUnmounted } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { getCurrentWindow } from '@tauri-apps/api/window';

const appWindow = getCurrentWindow();
const version = ref('');
const notes = ref('');
const status = ref<'idle' | 'downloading' | 'animating'>('idle');
const progress = ref(0);
let unlistenProgress: any = null;

onMounted(async () => {
  // Load data from localStorage
  const stored = localStorage.getItem('spellcoach_update_data');
  if (stored) {
    try {
      const parsed = JSON.parse(stored);
      version.value = parsed.version || '';
      notes.value = parsed.notes || 'Pequenas correções e melhorias.';
    } catch (e) {
      console.error(e);
    }
  }

  // Listen to download progress
  unlistenProgress = await listen('update-progress', (event: any) => {
    const { chunk, total } = event.payload;
    if (total > 0) {
      // Accumulate chunk over total since chunk is probably bytes downloaded so far,
      // actually tauri updater provides downloaded bytes (chunk is current bytes, total is total)
      // The payload structure is usually chunk = current downloaded length, but we can do a safe calc.
      // Assuming event.payload.chunk is chunk size and we need to accumulate, or it is already downloaded size.
      // Wait, in Rust we passed `chunk, total`. The updater callback gives `chunk_length, content_length`.
      // We need to accumulate the chunks.
      progress.value += chunk;
      const pct = Math.min(100, Math.round((progress.value / total) * 100));
      
      // If it hits 100%, trigger the nexus animation!
      if (pct >= 100 && status.value === 'downloading') {
        triggerNexusExplosion();
      }
    }
  });
});

onUnmounted(() => {
  if (unlistenProgress) unlistenProgress();
});

const closeWindow = () => {
  appWindow.hide();
};

const installUpdate = async () => {
  status.value = 'downloading';
  progress.value = 0;
  try {
    // We invoke the command but don't await immediately to allow UI updates
    invoke('download_and_install_update').catch(e => {
      console.error('Update failed:', e);
      status.value = 'idle';
      alert("Falha ao atualizar: " + e);
    });
  } catch (e) {
    console.error(e);
  }
};

const triggerNexusExplosion = () => {
  status.value = 'animating';
  // The animation CSS takes over. The Rust backend will restart the app after the install finishes.
  // The install takes a few seconds, so the animation plays out while it finishes installing.
};

const formatNotes = (text: string) => {
  // Simple markdown-ish to HTML for display
  let html = text.replace(/\n/g, '<br/>');
  html = html.replace(/\*\*(.*?)\*\*/g, '<strong>$1</strong>');
  return html;
};

// Calculate progress percentage for CSS width
const progressPct = () => {
  // Try to use a fake total if we don't know it, but usually total is known
  // Here we just use the calculated % or a visual trick. Since we are accumulating chunk, 
  // we actually need the total from the event. But wait, `chunk` in Rust is `chunk_length`.
  // Let's use a dynamic width based on status if we don't have perfect total.
  // But wait, the computed pct inside the listener updates the progress.
  return status.value === 'downloading' ? 'loading' : '';
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
          <span>📝 Notas de Atualização</span>
        </div>
        <div class="changelog-content" v-html="formatNotes(notes)"></div>
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
              <div class="progress-bar" :class="progressPct()"></div>
            </div>
            <span class="progress-text">Baixando recursos hextech...</span>
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
}
.changelog-content {
  flex: 1;
  padding: 12px;
  overflow-y: auto;
  font-size: 13px;
  line-height: 1.6;
  color: #a09b8c;
}

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
