<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { getCurrentWindow } from '@tauri-apps/api/window';

const appWindow = getCurrentWindow();
const version = ref('');
const rawNotes = ref('');
const status = ref<'idle' | 'downloading' | 'animating' | 'error'>('idle');
const errorMsg = ref('');
const downloadPct = ref(0);
const hasTotal = ref(false);
let accumulatedBytes = 0;
let totalBytes = 0;
let unlistenProgress: any = null;
let unlistenInstalling: any = null;

interface ReleaseChange { type: 'novo' | 'melhoria' | 'fix'; text: string; }
interface Release { version: string; date: string; changes: ReleaseChange[]; }

const currentRelease = ref<Release | null>(null);
const previousReleases = ref<Release[]>([]);

const notesFallbackLines = computed(() =>
  rawNotes.value.split('\n').map(l => l.replace(/^[-*•]\s*/, '').trim()).filter(l => l.length > 0)
);

async function loadChangelog(targetVersion: string) {
  let data: Release[] | null = null;
  try {
    const r = await fetch('https://raw.githubusercontent.com/SpellerBarbosa/riftsyncai/main/public/releases.json', { signal: AbortSignal.timeout(5000) });
    if (r.ok) data = await r.json();
  } catch (_) {}
  if (!data) {
    try { const r = await fetch('/releases.json'); if (r.ok) data = await r.json(); } catch (_) {}
  }
  if (!data || data.length === 0) return;
  const idx = targetVersion ? data.findIndex(r => r.version === targetVersion) : -1;
  currentRelease.value = idx >= 0 ? data[idx] : data[0];
  previousReleases.value = data.filter(r => r.version !== currentRelease.value?.version).slice(0, 4);
}

onMounted(async () => {
  const stored = localStorage.getItem('spellcoach_update_data');
  if (stored) {
    try { const p = JSON.parse(stored); version.value = p.version || ''; rawNotes.value = p.notes || ''; } catch (_) {}
  }
  await loadChangelog(version.value);

  unlistenProgress = await listen('update-progress', (e: any) => {
    const chunk: number = e.payload.chunk ?? 0;
    const total: number | null = e.payload.total ?? null;
    accumulatedBytes += chunk;
    if (total && total > 0) {
      hasTotal.value = true; totalBytes = total;
      downloadPct.value = Math.min(99, Math.round((accumulatedBytes / totalBytes) * 100));
    }
  });

  unlistenInstalling = await listen('update-installing', () => {
    downloadPct.value = 100;
    status.value = 'animating';
  });
});

onUnmounted(() => {
  if (unlistenProgress) unlistenProgress();
  if (unlistenInstalling) unlistenInstalling();
});

const closeWindow = () => appWindow.hide();

const installUpdate = async () => {
  status.value = 'downloading';
  accumulatedBytes = 0; totalBytes = 0; downloadPct.value = 0; hasTotal.value = false;
  invoke('download_and_install_update').catch((e: any) => {
    errorMsg.value = String(e);
    status.value = 'error';
  });
};

const typeLabel: Record<string, string> = { novo: 'NOVO', melhoria: 'MELHORIA', fix: 'FIX' };
</script>

<template>
  <div class="updater-root">

    <!-- Background layers -->
    <div class="bg-base"></div>
    <div class="bg-glow-top"></div>
    <div class="bg-glow-bottom"></div>

    <!-- Outer frame border -->
    <div class="frame-border"></div>

    <!-- Corner ornaments (L-shaped gold accents) -->
    <div class="orn-corner orn-tl"></div>
    <div class="orn-corner orn-tr"></div>
    <div class="orn-corner orn-bl"></div>
    <div class="orn-corner orn-br"></div>

    <!-- ══════════════════════════════════════════════
         INSTALL ANIMATION OVERLAY
         ══════════════════════════════════════════════ -->
    <transition name="overlay-fade">
      <div v-if="status === 'animating'" class="install-overlay">

        <!-- Expanding ring pulses -->
        <div class="pulse-rings">
          <div class="pulse-ring pr1"></div>
          <div class="pulse-ring pr2"></div>
          <div class="pulse-ring pr3"></div>
        </div>

        <!-- Hextech crystal -->
        <div class="crystal-wrap">
          <div class="crystal-halo"></div>
          <svg class="crystal-svg" width="52" height="60" viewBox="0 0 52 60" fill="none" xmlns="http://www.w3.org/2000/svg">
            <!-- Outer hex -->
            <polygon points="26,2 50,15 50,45 26,58 2,45 2,15"
                     stroke="#0397ab" stroke-width="1.5"
                     fill="rgba(3,151,171,0.07)"/>
            <!-- Mid hex -->
            <polygon points="26,10 42,19 42,41 26,50 10,41 10,19"
                     stroke="#cdfafa" stroke-width="1"
                     fill="rgba(3,151,171,0.18)"/>
            <!-- Inner hex (bright core) -->
            <polygon points="26,19 36,24.5 36,36 26,41.5 16,36 16,24.5"
                     fill="rgba(205,250,250,0.55)"/>
            <!-- Connector lines -->
            <line x1="26" y1="2"  x2="26" y2="10"  stroke="rgba(205,250,250,0.5)" stroke-width="0.75"/>
            <line x1="50" y1="15" x2="42" y2="19"  stroke="rgba(205,250,250,0.5)" stroke-width="0.75"/>
            <line x1="50" y1="45" x2="42" y2="41"  stroke="rgba(205,250,250,0.5)" stroke-width="0.75"/>
            <line x1="26" y1="58" x2="26" y2="50"  stroke="rgba(205,250,250,0.5)" stroke-width="0.75"/>
            <line x1="2"  y1="45" x2="10" y2="41"  stroke="rgba(205,250,250,0.5)" stroke-width="0.75"/>
            <line x1="2"  y1="15" x2="10" y2="19"  stroke="rgba(205,250,250,0.5)" stroke-width="0.75"/>
          </svg>
          <div class="crystal-flash"></div>
        </div>

        <p class="install-label">INSTALANDO NOVA VERSÃO...</p>

      </div>
    </transition>

    <!-- ══════════════════════════════════════════════
         MAIN CONTENT
         ══════════════════════════════════════════════ -->
    <div class="main-content" :class="{ 'content-hidden': status === 'animating' }">

      <!-- ── HEADER ──────────────────────────────── -->
      <header class="lol-header">

        <!-- Top ornamental divider -->
        <div class="orn-divider">
          <div class="orn-div-line"></div>
          <span class="orn-div-gem">◆</span>
          <div class="orn-div-line"></div>
        </div>

        <!-- Icon + titles -->
        <div class="header-icon-wrap">
          <svg width="32" height="32" viewBox="0 0 32 32" fill="none" xmlns="http://www.w3.org/2000/svg">
            <polygon points="16,2 30,9 30,23 16,30 2,23 2,9"
                     stroke="#c8aa6e" stroke-width="1.5" fill="rgba(200,170,110,0.08)"/>
            <polygon points="16,7 25,12 25,21 16,26 7,21 7,12"
                     fill="rgba(200,170,110,0.12)" stroke="#c8aa6e" stroke-width="1"/>
            <polygon points="16,12 21,15 21,19 16,22 11,19 11,15"
                     fill="rgba(200,170,110,0.6)"/>
          </svg>
        </div>

        <p class="header-app-name">SPELL COACH IA</p>
        <h1 class="header-title">Atualização Disponível</h1>

        <!-- Version pill (hexagonal) -->
        <div class="version-pill">
          <span class="version-pill-label">VERSÃO</span>
          <span class="version-pill-number">{{ version }}</span>
        </div>

        <!-- Bottom ornamental divider -->
        <div class="orn-divider orn-divider-sm">
          <div class="orn-div-line"></div>
          <span class="orn-div-gem orn-div-gem-sm">◆</span>
          <div class="orn-div-line"></div>
        </div>

      </header>

      <!-- ── CHANGELOG PANEL ────────────────────── -->
      <div class="changelog-panel">

        <!-- Panel header bar -->
        <div class="panel-bar">
          <div class="panel-bar-accent"></div>
          <span class="panel-bar-title">NOTAS DA VERSÃO</span>
          <span v-if="currentRelease" class="panel-bar-date">{{ currentRelease.date }}</span>
        </div>

        <!-- Scrollable content -->
        <div class="cl-scroll">

          <template v-if="currentRelease">
            <div v-for="(c, i) in currentRelease.changes" :key="i" class="note-row">
              <span class="note-badge" :class="`nb-${c.type}`">{{ typeLabel[c.type] }}</span>
              <span class="note-text">{{ c.text }}</span>
            </div>
          </template>

          <template v-else-if="notesFallbackLines.length">
            <div v-for="(line, i) in notesFallbackLines" :key="i" class="note-row">
              <span class="note-bullet">◆</span>
              <span class="note-text">{{ line }}</span>
            </div>
          </template>

          <p v-else class="note-empty">Pequenas correções e melhorias.</p>

          <template v-if="previousReleases.length">
            <div class="prev-sep">
              <div class="prev-sep-line"></div>
              <span>VERSÕES ANTERIORES</span>
              <div class="prev-sep-line"></div>
            </div>
            <template v-for="rel in previousReleases" :key="rel.version">
              <div class="prev-version-row">
                <span class="prev-ver-num">v{{ rel.version }}</span>
                <span class="prev-ver-date">{{ rel.date }}</span>
              </div>
              <div v-for="(c, i) in rel.changes" :key="i" class="note-row note-row-dim">
                <span class="note-badge" :class="`nb-${c.type}`">{{ typeLabel[c.type] }}</span>
                <span class="note-text">{{ c.text }}</span>
              </div>
            </template>
          </template>

        </div>
      </div>

      <!-- ── ACTION FOOTER ─────────────────────── -->
      <footer class="action-footer">

        <!-- IDLE: two buttons -->
        <template v-if="status === 'idle'">
          <button class="lol-btn lol-btn-ghost" @click="closeWindow">
            Lembrar Depois
          </button>
          <button class="lol-btn lol-btn-primary" @click="installUpdate">
            <svg width="13" height="13" viewBox="0 0 13 13" fill="none" class="btn-icon">
              <path d="M6.5 1v8M3 6l3.5 3.5L10 6M1.5 12h10"
                    stroke="currentColor" stroke-width="1.5"
                    stroke-linecap="round" stroke-linejoin="round"/>
            </svg>
            Instalar e Reiniciar
          </button>
        </template>

        <!-- DOWNLOADING: progress bar -->
        <div v-else-if="status === 'downloading'" class="dl-wrap">
          <div class="dl-info-row">
            <span class="dl-label">
              {{ hasTotal ? 'Baixando atualização...' : 'Baixando recursos hextech...' }}
            </span>
            <span v-if="hasTotal" class="dl-pct">{{ downloadPct }}%</span>
          </div>
          <div class="progress-track">
            <div v-if="hasTotal"
                 class="progress-fill"
                 :style="{ width: downloadPct + '%' }">
              <div class="progress-shine"></div>
            </div>
            <div v-else class="progress-indeterm"></div>
          </div>
        </div>

        <!-- ERROR state -->
        <div v-else-if="status === 'error'" class="error-wrap">
          <span class="error-icon">⚠</span>
          <p class="error-msg">{{ errorMsg }}</p>
          <button class="lol-btn lol-btn-ghost error-retry-btn" @click="status = 'idle'">
            Tentar Novamente
          </button>
        </div>

      </footer>
    </div>

  </div>
</template>

<style scoped>

/* ================================================================
   ROOT
   ================================================================ */
.updater-root {
  position: fixed;
  inset: 0;
  display: flex;
  flex-direction: column;
  overflow: hidden;
  font-family: 'Segoe UI', system-ui, -apple-system, sans-serif;
  color: #f0e6d2;
  user-select: none;
}

/* ================================================================
   BACKGROUNDS
   ================================================================ */
.bg-base {
  position: absolute;
  inset: 0;
  background: #010a13;
  z-index: 0;
}

.bg-glow-top {
  position: absolute;
  inset: 0;
  background: radial-gradient(ellipse 90% 45% at 50% -8%,
    rgba(3, 151, 171, 0.22) 0%,
    transparent 100%);
  z-index: 0;
}

.bg-glow-bottom {
  position: absolute;
  inset: 0;
  background: radial-gradient(ellipse 60% 30% at 50% 108%,
    rgba(120, 90, 40, 0.12) 0%,
    transparent 100%);
  z-index: 0;
}

/* ================================================================
   FRAME + CORNER ORNAMENTS
   ================================================================ */
.frame-border {
  position: absolute;
  inset: 0;
  border: 1px solid #785a28;
  pointer-events: none;
  z-index: 2;
}

/* Gold L-shaped corners that overlay the thin frame border */
.orn-corner {
  position: absolute;
  width: 16px;
  height: 16px;
  z-index: 3;
  pointer-events: none;
}
.orn-corner::before,
.orn-corner::after {
  content: '';
  position: absolute;
  background: #c8aa6e;
}

.orn-tl { top: 0; left: 0; }
.orn-tl::before { top: 0; left: 0; width: 16px; height: 2px; }
.orn-tl::after  { top: 0; left: 0; width: 2px;  height: 16px; }

.orn-tr { top: 0; right: 0; }
.orn-tr::before { top: 0; right: 0; width: 16px; height: 2px; }
.orn-tr::after  { top: 0; right: 0; width: 2px;  height: 16px; }

.orn-bl { bottom: 0; left: 0; }
.orn-bl::before { bottom: 0; left: 0; width: 16px; height: 2px; }
.orn-bl::after  { bottom: 0; left: 0; width: 2px;  height: 16px; }

.orn-br { bottom: 0; right: 0; }
.orn-br::before { bottom: 0; right: 0; width: 16px; height: 2px; }
.orn-br::after  { bottom: 0; right: 0; width: 2px;  height: 16px; }

/* ================================================================
   INSTALL OVERLAY
   ================================================================ */
.install-overlay {
  position: absolute;
  inset: 0;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  z-index: 30;
  background: rgba(1, 10, 19, 0.75);
  backdrop-filter: blur(4px);
}

/* Pulse rings */
.pulse-rings {
  position: absolute;
  width: 120px;
  height: 120px;
  display: flex;
  align-items: center;
  justify-content: center;
  margin-bottom: 0;
}

.pulse-ring {
  position: absolute;
  border-radius: 50%;
  border: 1px solid rgba(3, 151, 171, 0.7);
  animation: ringPulse 2.4s ease-out infinite;
}
.pr1 { width: 50px;  height: 50px;  animation-delay: 0s; }
.pr2 { width: 82px;  height: 82px;  animation-delay: 0.7s; }
.pr3 { width: 114px; height: 114px; animation-delay: 1.4s; }

/* Crystal */
.crystal-wrap {
  position: relative;
  width: 80px;
  height: 80px;
  display: flex;
  align-items: center;
  justify-content: center;
  margin-bottom: 22px;
  z-index: 2;
}

.crystal-halo {
  position: absolute;
  inset: -20px;
  border-radius: 50%;
  background: radial-gradient(circle, rgba(3, 151, 171, 0.28) 0%, transparent 70%);
  animation: haloPulse 1.6s ease-in-out infinite alternate;
}

.crystal-svg {
  position: relative;
  z-index: 3;
  animation: crystalFloat 1.2s ease-in-out infinite alternate,
             crystalGlow  1.6s ease-in-out infinite;
  filter: drop-shadow(0 0 10px rgba(3, 151, 171, 0.8));
}

.crystal-flash {
  position: absolute;
  inset: -40px;
  background: radial-gradient(circle, rgba(3, 205, 250, 0.2) 0%, transparent 65%);
  border-radius: 50%;
  z-index: 1;
  animation: flashPulse 1s ease-in-out infinite alternate;
}

.install-label {
  font-size: 10px;
  font-weight: 700;
  letter-spacing: 0.32em;
  color: #0397ab;
  text-shadow: 0 0 14px rgba(3, 205, 250, 0.9);
  animation: labelBlink 1.1s ease-in-out infinite;
  margin: 0;
}

@keyframes ringPulse {
  0%   { transform: scale(0.2); opacity: 0.9; }
  85%  { opacity: 0.1; }
  100% { transform: scale(2.2); opacity: 0; }
}
@keyframes haloPulse  { from { opacity: 0.5; transform: scale(0.85); } to { opacity: 1; transform: scale(1.15); } }
@keyframes crystalFloat { from { transform: translateY(-5px) rotate(-4deg); } to { transform: translateY(5px) rotate(4deg); } }
@keyframes crystalGlow  { 0%,100% { filter: drop-shadow(0 0 8px rgba(3,151,171,0.6)); } 50% { filter: drop-shadow(0 0 22px rgba(3,225,255,1)); } }
@keyframes flashPulse   { from { opacity: 0.2; transform: scale(0.8); } to { opacity: 0.6; transform: scale(1.2); } }
@keyframes labelBlink   { 0%,100% { opacity: 0.5; } 50% { opacity: 1; } }

/* Overlay fade transition */
.overlay-fade-enter-active,
.overlay-fade-leave-active { transition: opacity 0.4s ease; }
.overlay-fade-enter-from,
.overlay-fade-leave-to { opacity: 0; }

/* ================================================================
   MAIN CONTENT
   ================================================================ */
.main-content {
  position: relative;
  display: flex;
  flex-direction: column;
  flex: 1;
  min-height: 0;
  padding: 0 20px;
  z-index: 10;
  transition: opacity 0.4s ease;
}

.content-hidden {
  opacity: 0;
  pointer-events: none;
}

/* ================================================================
   HEADER
   ================================================================ */
.lol-header {
  flex-shrink: 0;
  text-align: center;
  padding: 16px 0 10px;
}

/* Ornamental dividers */
.orn-divider {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-bottom: 12px;
}
.orn-divider-sm {
  margin-top: 12px;
  margin-bottom: 0;
}

.orn-div-line {
  flex: 1;
  height: 1px;
  background: linear-gradient(90deg, transparent, #785a28 40%, #785a28 60%, transparent);
}

.orn-div-gem {
  color: #c8aa6e;
  font-size: 10px;
  line-height: 1;
  filter: drop-shadow(0 0 5px rgba(200, 170, 110, 0.7));
}
.orn-div-gem-sm { font-size: 7px; }

/* Header icon */
.header-icon-wrap {
  display: flex;
  justify-content: center;
  margin-bottom: 6px;
  filter: drop-shadow(0 0 8px rgba(200, 170, 110, 0.4));
}

.header-app-name {
  font-size: 8px;
  letter-spacing: 0.38em;
  color: #785a28;
  font-weight: 700;
  text-transform: uppercase;
  margin: 0 0 5px;
}

.header-title {
  font-size: 17px;
  font-weight: 700;
  color: #c8aa6e;
  letter-spacing: 0.1em;
  text-transform: uppercase;
  margin: 0 0 10px;
  text-shadow: 0 0 24px rgba(200, 170, 110, 0.35);
}

/* Hexagonal version pill */
.version-pill {
  display: inline-flex;
  align-items: center;
  gap: 7px;
  padding: 4px 16px;
  background: rgba(3, 151, 171, 0.08);
  box-shadow: inset 0 0 0 1px rgba(3, 151, 171, 0.38);
  clip-path: polygon(
    12px 0%,
    calc(100% - 12px) 0%,
    100% 50%,
    calc(100% - 12px) 100%,
    12px 100%,
    0% 50%
  );
}

.version-pill-label {
  font-size: 7.5px;
  letter-spacing: 0.22em;
  color: #0397ab;
  font-weight: 700;
}

.version-pill-number {
  font-size: 13px;
  font-weight: 700;
  color: #cdfafa;
  text-shadow: 0 0 10px rgba(3, 205, 250, 0.55);
}

/* ================================================================
   CHANGELOG PANEL
   ================================================================ */
.changelog-panel {
  flex: 1;
  min-height: 0;
  display: flex;
  flex-direction: column;
  border: 1px solid rgba(120, 90, 40, 0.4);
  background: rgba(0, 0, 0, 0.28);
  overflow: hidden;
}

/* Panel header bar */
.panel-bar {
  flex-shrink: 0;
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 6px 10px;
  border-bottom: 1px solid rgba(120, 90, 40, 0.28);
  background: rgba(200, 170, 110, 0.045);
}

.panel-bar-accent {
  width: 3px;
  height: 11px;
  background: linear-gradient(180deg, #c8aa6e, #785a28);
  flex-shrink: 0;
}

.panel-bar-title {
  flex: 1;
  font-size: 8.5px;
  font-weight: 700;
  letter-spacing: 0.28em;
  color: #c8aa6e;
}

.panel-bar-date {
  font-size: 8.5px;
  color: #3a3a3a;
}

/* Scrollable area */
.cl-scroll {
  flex: 1;
  min-height: 0;
  overflow-y: auto;
  padding: 8px 10px;
  display: flex;
  flex-direction: column;
  gap: 5px;
}

.cl-scroll::-webkit-scrollbar { width: 4px; }
.cl-scroll::-webkit-scrollbar-track { background: transparent; }
.cl-scroll::-webkit-scrollbar-thumb { background: rgba(200, 170, 110, 0.28); border-radius: 2px; }

/* Note rows */
.note-row {
  display: flex;
  align-items: flex-start;
  gap: 6px;
  font-size: 11px;
}

.note-row-dim { opacity: 0.38; }

/* Hexagonal badges */
.note-badge {
  flex-shrink: 0;
  font-size: 7px;
  font-weight: 700;
  letter-spacing: 0.04em;
  padding: 1px 6px;
  margin-top: 1px;
  clip-path: polygon(
    4px 0%, calc(100% - 4px) 0%,
    100% 50%,
    calc(100% - 4px) 100%, 4px 100%,
    0% 50%
  );
}

.nb-novo     { background: rgba(74, 240, 118, 0.14); color: #4af076; }
.nb-melhoria { background: rgba(74, 180, 240, 0.14); color: #4ab4f0; }
.nb-fix      { background: rgba(240, 168, 74, 0.14);  color: #f0a84a; }

.note-bullet {
  flex-shrink: 0;
  font-size: 6px;
  color: #c8aa6e;
  margin-top: 3px;
}

.note-text  { color: #c8b89a; line-height: 1.45; }
.note-empty { font-size: 11px; color: #3a3a3a; margin: 0; }

/* Previous versions separator */
.prev-sep {
  display: flex;
  align-items: center;
  gap: 6px;
  font-size: 8px;
  letter-spacing: 0.18em;
  color: #2c2c2c;
  margin: 8px 0 4px;
}
.prev-sep-line { flex: 1; height: 1px; background: rgba(44, 44, 44, 0.8); }

.prev-version-row {
  display: flex;
  align-items: center;
  gap: 6px;
  margin-bottom: 2px;
}
.prev-ver-num  { font-size: 9px; font-weight: 700; color: #3a3a3a; }
.prev-ver-date { font-size: 8px; color: #2a2a2a; }

/* ================================================================
   ACTION FOOTER
   ================================================================ */
.action-footer {
  flex-shrink: 0;
  padding: 11px 0 16px;
  display: flex;
  gap: 10px;
  align-items: center;
}

/* ── LOL-STYLE BUTTONS (clip-path cut corners) ── */
.lol-btn {
  flex: 1;
  height: 40px;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: 0;
  font-size: 11px;
  font-weight: 700;
  letter-spacing: 0.09em;
  text-transform: uppercase;
  cursor: pointer;
  border: none;
  outline: none;
  transition: box-shadow 0.18s, filter 0.18s, background 0.18s;
  clip-path: polygon(
    8px 0%, calc(100% - 8px) 0%,
    100% 8px,
    100% calc(100% - 8px),
    calc(100% - 8px) 100%, 8px 100%,
    0% calc(100% - 8px),
    0% 8px
  );
}

.btn-icon {
  margin-right: 6px;
  flex-shrink: 0;
}

.lol-btn-ghost {
  background: rgba(0, 0, 0, 0.5);
  color: #7a7570;
  box-shadow: inset 0 0 0 1px rgba(120, 90, 40, 0.45);
}

.lol-btn-ghost:hover {
  color: #c8aa6e;
  background: rgba(200, 170, 110, 0.06);
  box-shadow: inset 0 0 0 1px #c8aa6e;
}

.lol-btn-primary {
  background: linear-gradient(180deg,
    #1c3545 0%,
    #0d1e2e 45%,
    #010a13 100%
  );
  color: #f0e6d2;
  box-shadow:
    inset 0 0 0 1px #785a28,
    0 0 14px rgba(200, 170, 110, 0.1);
  position: relative;
  overflow: hidden;
}

.lol-btn-primary::after {
  content: '';
  position: absolute;
  top: 0;
  left: -100%;
  width: 50%;
  height: 100%;
  background: linear-gradient(90deg,
    transparent,
    rgba(255, 255, 255, 0.12),
    transparent
  );
  transform: skewX(-20deg);
  animation: btnSweep 3.5s infinite;
}

.lol-btn-primary:hover {
  box-shadow:
    inset 0 0 0 1px #c8aa6e,
    0 0 24px rgba(200, 170, 110, 0.25);
  filter: brightness(1.1);
}

@keyframes btnSweep {
  0%   { left: -100%; }
  22%  { left: 200%; }
  100% { left: 200%; }
}

/* ── DOWNLOAD STATE ── */
.dl-wrap {
  flex: 1;
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.dl-info-row {
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.dl-label {
  font-size: 11px;
  color: #a09b8c;
  letter-spacing: 0.02em;
}

.dl-pct {
  font-size: 13px;
  font-weight: 700;
  color: #c8aa6e;
}

.progress-track {
  width: 100%;
  height: 5px;
  background: rgba(0, 0, 0, 0.55);
  border: 1px solid rgba(200, 170, 110, 0.18);
  overflow: hidden;
  position: relative;
}

.progress-fill {
  height: 100%;
  background: linear-gradient(90deg, #785a28 0%, #c8aa6e 80%, #f0d878 100%);
  box-shadow: 0 0 8px rgba(200, 170, 110, 0.6);
  transition: width 0.2s ease;
  position: relative;
  overflow: hidden;
}

.progress-shine {
  position: absolute;
  inset: 0;
  background: linear-gradient(90deg, transparent 60%, rgba(255, 255, 255, 0.25) 80%, transparent 100%);
  animation: shineSweep 1.8s linear infinite;
}

@keyframes shineSweep {
  from { transform: translateX(-100%); }
  to   { transform: translateX(200%); }
}

.progress-indeterm {
  position: absolute;
  inset: 0;
  background: linear-gradient(90deg, transparent 0%, #c8aa6e 50%, transparent 100%);
  background-size: 200% 100%;
  animation: indetermAnim 1.6s linear infinite;
}

@keyframes indetermAnim {
  0%   { background-position: 200% 0; }
  100% { background-position: -200% 0; }
}

/* ── ERROR STATE ── */
.error-wrap {
  flex: 1;
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 5px;
}

.error-icon {
  font-size: 20px;
  color: #f04a4a;
  text-shadow: 0 0 10px rgba(240, 74, 74, 0.5);
}

.error-msg {
  font-size: 10px;
  color: #f04a4a;
  text-align: center;
  margin: 0;
  word-break: break-word;
  max-width: 100%;
}

.error-retry-btn {
  margin-top: 2px;
  flex: 0 0 auto;
  width: auto;
  padding: 0 20px;
}

</style>
