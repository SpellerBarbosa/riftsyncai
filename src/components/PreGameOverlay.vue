<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted } from 'vue';
import { getCurrentWindow } from '@tauri-apps/api/window';

interface Matchup {
  enemy: string;
  role: string;
  win_rate: number;
  verdict: 'win' | 'avoid' | 'even' | 'unknown';
}

interface PreGameReport {
  player_champion: string;
  player_role: string;
  matchups: Matchup[];
  avoid_1v1: string[];
}

const appWindow = getCurrentWindow();
const report = ref<PreGameReport | null>(null);
const countdown = ref(50);
let timer: ReturnType<typeof setInterval> | null = null;

const roleLabel: Record<string, string> = {
  TOP: 'Top', JUNGLE: 'JG', MIDDLE: 'Mid', BOTTOM: 'ADC', UTILITY: 'Sup',
};

const displayRole = computed(() => {
  if (!report.value) return '';
  return roleLabel[report.value.player_role.toUpperCase()] ?? report.value.player_role;
});

const verdictIcon = (v: string) => ({ win: '✅', avoid: '❌', even: '⚠️' }[v] ?? '—');
const verdictLabel = (v: string) => ({ win: 'você ganha', avoid: 'evite 1v1', even: 'equilibrado', unknown: 'sem dados' }[v] ?? '—');
const wrDisplay = (wr: number, verdict: string) => verdict === 'unknown' ? '' : `${(wr * 100).toFixed(0)}%`;

const close = () => appWindow.close();
const onKey = (e: KeyboardEvent) => { if (e.key === 'Escape') close(); };

onMounted(async () => {
  // Garante que cliques funcionem mesmo com o loading screen em foco
  await appWindow.setIgnoreCursorEvents(false);

  const stored = localStorage.getItem('spellcoach_pregame');
  if (stored) {
    try { report.value = JSON.parse(stored); } catch (_) {}
  }

  timer = setInterval(() => {
    countdown.value--;
    if (countdown.value <= 0) close();
  }, 1000);

  window.addEventListener('keydown', onKey);
});

onUnmounted(() => {
  if (timer) clearInterval(timer);
  window.removeEventListener('keydown', onKey);
});
</script>

<template>
  <div v-if="report" class="pregame">
    <!-- Header -->
    <div class="header" data-tauri-drag-region>
      <div class="header-left" data-tauri-drag-region>
        <span class="badge">ANÁLISE DE DRAFT</span>
        <span class="divider">•</span>
        <span class="champ-label">{{ report.player_champion }}</span>
        <span class="role-label">{{ displayRole }}</span>
      </div>
      <button class="close-btn" @click="close" title="Fechar (ESC)">×</button>
    </div>

    <!-- Matchups -->
    <div class="matchups-section">
      <div class="section-title">MATCHUPS 1v1</div>
      <div class="matchups-list">
        <div
          v-for="m in report.matchups"
          :key="m.enemy"
          class="matchup-row"
          :class="m.verdict"
        >
          <span class="verdict-icon">{{ verdictIcon(m.verdict) }}</span>
          <span class="enemy-name">{{ m.enemy }}</span>
          <span class="role-pill">{{ m.role }}</span>
          <span class="wr-badge" :class="m.verdict">{{ wrDisplay(m.win_rate, m.verdict) }}</span>
          <span class="verdict-text">{{ verdictLabel(m.verdict) }}</span>
        </div>
      </div>
    </div>

    <!-- Tip -->
    <div v-if="report.avoid_1v1.length > 0" class="avoid-banner">
      <span class="avoid-icon">⚠️</span>
      <span>Jogue sempre em <strong>2 ou mais</strong> contra <strong>{{ report.avoid_1v1.join(', ') }}</strong></span>
    </div>

    <!-- Footer -->
    <div class="footer">
      <div class="footer-line"></div>
      <div class="footer-content">
        <span class="brand">SPELL COACH IA</span>
        <button class="esc-btn" @click="close">ESC — fechar</button>
        <span class="countdown">{{ countdown }}s</span>
      </div>
    </div>
  </div>
</template>

<style scoped>
.pregame {
  width: 100%;
  height: 100%;
  display: flex;
  flex-direction: column;
  background: linear-gradient(160deg, rgba(1,8,16,0.98) 0%, rgba(2,12,22,0.99) 100%);
  border: 1px solid #c8aa6e;
  border-radius: 6px;
  overflow: hidden;
  font-family: 'Segoe UI', sans-serif;
  color: #f0e6d2;
  box-shadow: 0 20px 60px rgba(0,0,0,0.95), inset 0 0 40px rgba(0,0,0,0.4);
}

/* ── HEADER ──────────────────────────────────────────────── */
.header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 9px 12px 8px;
  border-bottom: 1px solid rgba(200,170,110,0.2);
  background: rgba(200,170,110,0.04);
}

.header-left {
  display: flex;
  align-items: center;
  gap: 7px;
}

.badge {
  font-size: 7.5px;
  font-weight: 800;
  letter-spacing: 1.5px;
  text-transform: uppercase;
  color: #c8aa6e;
  background: rgba(200,170,110,0.12);
  border: 1px solid rgba(200,170,110,0.4);
  padding: 2px 6px;
  border-radius: 2px;
}

.divider { color: rgba(200,170,110,0.3); font-size: 10px; }

.champ-label {
  font-size: 13px;
  font-weight: 900;
  color: #f0e6d2;
  letter-spacing: 0.5px;
  text-transform: uppercase;
}

.role-label {
  font-size: 10px;
  font-weight: 700;
  color: #a09b8c;
  background: rgba(255,255,255,0.05);
  padding: 1px 6px;
  border-radius: 3px;
}

.close-btn {
  background: none;
  border: none;
  color: #a09b8c;
  font-size: 18px;
  line-height: 1;
  cursor: pointer;
  padding: 2px 4px;
  border-radius: 3px;
  transition: all 0.15s;
}
.close-btn:hover { color: #ff4e4e; background: rgba(255,78,78,0.12); }

/* ── MATCHUPS ────────────────────────────────────────────── */
.matchups-section {
  flex: 1;
  padding: 10px 12px 6px;
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.section-title {
  font-size: 7.5px;
  font-weight: 800;
  letter-spacing: 2px;
  color: #5b5a56;
  text-transform: uppercase;
  margin-bottom: 2px;
}

.matchups-list {
  display: flex;
  flex-direction: column;
  gap: 3px;
}

.matchup-row {
  display: grid;
  grid-template-columns: 18px 1fr 34px 38px 90px;
  align-items: center;
  gap: 8px;
  padding: 5px 8px;
  border-radius: 4px;
  border-left: 2px solid transparent;
  background: rgba(255,255,255,0.02);
  transition: background 0.15s;
}

.matchup-row.win   { border-left-color: #4eff9b; background: rgba(78,255,155,0.04); }
.matchup-row.avoid { border-left-color: #ff4e4e; background: rgba(255,78,78,0.05); }
.matchup-row.even  { border-left-color: #c8aa6e; background: rgba(200,170,110,0.04); }

.verdict-icon { font-size: 11px; text-align: center; }

.enemy-name {
  font-size: 11.5px;
  font-weight: 700;
  color: #f0e6d2;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.role-pill {
  font-size: 8px;
  font-weight: 800;
  color: #5b5a56;
  text-transform: uppercase;
  letter-spacing: 0.5px;
  text-align: center;
}

.wr-badge {
  font-size: 10px;
  font-weight: 800;
  text-align: right;
}
.wr-badge.win   { color: #4eff9b; }
.wr-badge.avoid { color: #ff4e4e; }
.wr-badge.even  { color: #c8aa6e; }
.wr-badge.unknown { color: #5b5a56; }

.verdict-text {
  font-size: 9px;
  color: #5b5a56;
  white-space: nowrap;
}
.matchup-row.win   .verdict-text { color: #4eff9b; opacity: 0.8; }
.matchup-row.avoid .verdict-text { color: #ff4e4e; opacity: 0.8; }
.matchup-row.even  .verdict-text { color: #c8aa6e; opacity: 0.7; }

/* ── AVOID BANNER ────────────────────────────────────────── */
.avoid-banner {
  display: flex;
  align-items: center;
  gap: 8px;
  margin: 0 12px 8px;
  padding: 7px 10px;
  background: rgba(255,78,78,0.07);
  border: 1px solid rgba(255,78,78,0.25);
  border-radius: 4px;
  font-size: 11px;
  color: #f0e6d2;
  line-height: 1.4;
}
.avoid-icon { font-size: 13px; flex-shrink: 0; }
.avoid-banner strong { color: #ff7070; }

/* ── FOOTER ──────────────────────────────────────────────── */
.footer {
  padding: 0 12px 7px;
}

.footer-line {
  height: 1px;
  background: rgba(200,170,110,0.15);
  margin-bottom: 6px;
}

.footer-content {
  display: flex;
  align-items: center;
  justify-content: space-between;
}

.brand {
  font-size: 7px;
  font-weight: 800;
  letter-spacing: 1.5px;
  color: #3a3830;
  text-transform: uppercase;
}

.esc-btn {
  background: rgba(255,255,255,0.04);
  border: 1px solid rgba(255,255,255,0.08);
  border-radius: 3px;
  color: #5b5a56;
  font-size: 8.5px;
  font-weight: 700;
  padding: 2px 8px;
  cursor: pointer;
  letter-spacing: 0.5px;
  transition: all 0.15s;
}
.esc-btn:hover { background: rgba(255,78,78,0.12); border-color: rgba(255,78,78,0.4); color: #ff7070; }

.countdown {
  font-size: 9px;
  color: #3a3830;
  font-weight: 600;
  min-width: 24px;
  text-align: right;
}
</style>
