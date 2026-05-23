<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted } from 'vue';
import { getCurrentWindow } from '@tauri-apps/api/window';

interface Ally {
  name: string;
  champion_id: string;
  role: string;
  is_self: boolean;
}

interface Matchup {
  enemy: string;
  champion_id: string;
  role: string;
  win_rate: number;
  verdict: 'win' | 'avoid' | 'even' | 'unknown';
}

interface PreGameReport {
  player_champion: string;
  player_role: string;
  allies: Ally[];
  matchups: Matchup[];
  avoid_1v1: string[];
}

const appWindow = getCurrentWindow();
const report = ref<PreGameReport | null>(null);
const countdown = ref(50);
let timer: ReturnType<typeof setInterval> | null = null;

const DDRAGON_VERSION = '16.10.1';

function portraitUrl(id: string) {
  return `https://ddragon.leagueoflegends.com/cdn/img/champion/loading/${id}_0.jpg`;
}
function iconUrl(id: string) {
  return `https://ddragon.leagueoflegends.com/cdn/${DDRAGON_VERSION}/img/champion/${id}.png`;
}
function onImgError(e: Event, id: string) {
  (e.target as HTMLImageElement).src = iconUrl(id);
}

const roleLabel: Record<string, string> = {
  TOP: 'Top', JUNGLE: 'JG', MIDDLE: 'Mid', BOTTOM: 'ADC', UTILITY: 'Sup',
};
const displayRole = computed(() => {
  if (!report.value) return '';
  return roleLabel[report.value.player_role.toUpperCase()] ?? report.value.player_role;
});

const vc = {
  win:     { icon: '✓', label: 'Favorável',      color: '#4eff9b', glow: 'rgba(78,255,155,0.55)',  bg: 'rgba(78,255,155,0.13)'  },
  avoid:   { icon: '!', label: 'Evite 1v1',      color: '#ff4e4e', glow: 'rgba(255,78,78,0.55)',   bg: 'rgba(255,78,78,0.16)'   },
  even:    { icon: '~', label: 'Equilibrado',    color: '#c8aa6e', glow: 'rgba(200,170,110,0.45)', bg: 'rgba(200,170,110,0.11)' },
  unknown: { icon: '?', label: 'Sem dados',      color: '#5b5a56', glow: 'rgba(91,90,86,0.25)',   bg: 'rgba(30,30,30,0.5)'    },
} as const;

function vconf(v: string) { return vc[v as keyof typeof vc] ?? vc.unknown; }
function wrLabel(wr: number, v: string) { return v === 'unknown' ? '—' : `${(wr * 100).toFixed(0)}%`; }
function tacticalTag(m: Matchup): string {
  if (m.verdict === 'avoid') return 'Forte 1v1';
  if (m.verdict === 'win')   return m.win_rate >= 0.58 ? 'Fraco vs vc' : 'Vantagem';
  if (m.verdict === 'even')  return 'Disputa igual';
  return 'Sem histórico';
}

const close = () => appWindow.close();
const onKey = (e: KeyboardEvent) => { if (e.key === 'Escape') close(); };

onMounted(async () => {
  await appWindow.setIgnoreCursorEvents(false);
  const stored = localStorage.getItem('spellcoach_pregame');
  if (stored) { try { report.value = JSON.parse(stored); } catch (_) {} }
  timer = setInterval(() => { countdown.value--; if (countdown.value <= 0) close(); }, 1000);
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
        <span class="sep">•</span>
        <span class="champ-name">{{ report.player_champion }}</span>
        <span class="role-pill">{{ displayRole }}</span>
      </div>
      <div class="header-right">
        <span class="timer">{{ countdown }}s</span>
        <button class="x-btn" @click="close">×</button>
      </div>
    </div>

    <!-- Linha aliados -->
    <div class="team-row ally-row">
      <div class="team-label ally">SEU TIME</div>
      <div class="cards">
        <div
          v-for="a in report.allies"
          :key="a.champion_id"
          class="card"
          :class="{ 'is-self': a.is_self }"
        >
          <div class="portrait-wrap">
            <img
              :src="portraitUrl(a.champion_id)"
              :alt="a.name"
              class="portrait"
              draggable="false"
              @error="onImgError($event, a.champion_id)"
            />
            <div class="vignette ally-vignette"></div>
            <span class="role-tag">{{ a.role }}</span>
            <div v-if="a.is_self" class="self-crown">★ VOCÊ</div>
          </div>
          <span class="card-name">{{ a.name }}</span>
        </div>
      </div>
    </div>

    <!-- Divisor VS -->
    <div class="vs-divider">
      <div class="vs-line"></div>
      <span class="vs-text">VS</span>
      <div class="vs-line"></div>
    </div>

    <!-- Linha inimigos -->
    <div class="team-row enemy-row">
      <div class="team-label enemy">TIME INIMIGO</div>
      <div class="cards">
        <div
          v-for="m in report.matchups"
          :key="m.champion_id"
          class="card enemy-card"
          :class="m.verdict"
        >
          <div class="portrait-wrap">
            <img
              :src="portraitUrl(m.champion_id)"
              :alt="m.enemy"
              class="portrait"
              draggable="false"
              @error="onImgError($event, m.champion_id)"
            />
            <div
              class="vignette enemy-vignette"
              :style="{ '--vc': vconf(m.verdict).color }"
            ></div>
            <span class="role-tag">{{ m.role }}</span>
          </div>

          <!-- ORB badge com dica -->
          <div
            class="orb"
            :style="{
              '--vc': vconf(m.verdict).color,
              '--vg': vconf(m.verdict).glow,
              '--vb': vconf(m.verdict).bg,
            }"
          >
            <span class="orb-icon">{{ vconf(m.verdict).icon }}</span>
            <span class="orb-label">{{ tacticalTag(m) }}</span>
            <span class="orb-wr">{{ wrLabel(m.win_rate, m.verdict) }}</span>
          </div>

          <span class="card-name">{{ m.enemy }}</span>
        </div>
      </div>
    </div>

    <!-- Avoid banner -->
    <div v-if="report.avoid_1v1.length" class="avoid-banner">
      <span class="avoid-icon">⚠</span>
      <span>Sempre <strong>2+</strong> contra <strong>{{ report.avoid_1v1.join(', ') }}</strong></span>
    </div>

    <!-- Footer -->
    <div class="footer">
      <span class="brand">SPELL COACH IA</span>
      <button class="esc-btn" @click="close">ESC — fechar</button>
    </div>

  </div>
</template>

<style scoped>
* { box-sizing: border-box; margin: 0; padding: 0; }

.pregame {
  width: 100vw;
  height: 100vh;
  display: flex;
  flex-direction: column;
  background: linear-gradient(170deg, #010810 0%, #020c16 100%);
  border: 1px solid rgba(200,170,110,0.4);
  border-radius: 6px;
  overflow: hidden;
  font-family: 'Segoe UI', sans-serif;
  color: #f0e6d2;
  box-shadow: 0 24px 64px rgba(0,0,0,0.97);
}

/* ── HEADER ─────────────────────────────── */
.header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 7px 12px;
  border-bottom: 1px solid rgba(200,170,110,0.16);
  background: rgba(200,170,110,0.03);
  flex-shrink: 0;
}
.header-left { display: flex; align-items: center; gap: 7px; }
.header-right { display: flex; align-items: center; gap: 8px; }
.badge {
  font-size: 6.5px; font-weight: 800; letter-spacing: 1.5px;
  color: #c8aa6e; background: rgba(200,170,110,0.1);
  border: 1px solid rgba(200,170,110,0.3); padding: 2px 6px; border-radius: 2px;
}
.sep { color: rgba(200,170,110,0.3); font-size: 10px; }
.champ-name {
  font-size: 12px; font-weight: 900; color: #f0e6d2;
  letter-spacing: 0.5px; text-transform: uppercase;
}
.role-pill {
  font-size: 8.5px; font-weight: 700; color: #a09b8c;
  background: rgba(255,255,255,0.06); padding: 1px 6px; border-radius: 3px;
}
.timer { font-size: 9px; font-weight: 700; color: #5b5a56; }
.x-btn {
  background: none; border: none; color: #a09b8c; font-size: 18px;
  cursor: pointer; padding: 0 3px; border-radius: 3px; transition: all 0.15s;
  line-height: 1;
}
.x-btn:hover { color: #ff4e4e; background: rgba(255,78,78,0.12); }

/* ── TEAM ROW ───────────────────────────── */
.team-row {
  display: flex;
  align-items: flex-start;
  gap: 8px;
  padding: 6px 10px 0;
  flex: 1;
  min-height: 0;
}
.team-label {
  writing-mode: vertical-rl;
  text-orientation: mixed;
  transform: rotate(180deg);
  font-size: 6.5px;
  font-weight: 800;
  letter-spacing: 1.5px;
  text-transform: uppercase;
  padding: 4px 0;
  flex-shrink: 0;
  align-self: center;
}
.team-label.ally  { color: #4a9eff; opacity: 0.7; }
.team-label.enemy { color: #ff4e4e; opacity: 0.7; }

.cards {
  display: flex;
  gap: 5px;
  flex: 1;
  justify-content: center;
}

/* ── CARD ───────────────────────────────── */
.card {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 2px;
  flex: 1;
  max-width: 190px;
}

/* ── PORTRAIT ───────────────────────────── */
.portrait-wrap {
  position: relative;
  width: 100%;
  aspect-ratio: 3 / 4;
  border-radius: 3px;
  overflow: hidden;
  border: 1px solid rgba(255,255,255,0.07);
}
/* Aliado: borda azul sutil */
.ally-row .portrait-wrap        { border-color: rgba(74,158,255,0.25); }
.card.is-self .portrait-wrap    { border-color: rgba(200,170,110,0.6); box-shadow: 0 0 10px rgba(200,170,110,0.3); }
/* Inimigo: borda colorida por veredicto */
.enemy-card.win   .portrait-wrap { border-color: rgba(78,255,155,0.4); }
.enemy-card.avoid .portrait-wrap { border-color: rgba(255,78,78,0.45); }
.enemy-card.even  .portrait-wrap { border-color: rgba(200,170,110,0.3); }

.portrait {
  width: 100%; height: 100%;
  object-fit: cover; object-position: top center;
  display: block;
}
.ally-row .portrait   { filter: brightness(0.82) saturate(0.85); }
.card.is-self .portrait { filter: brightness(0.92) saturate(1.0); }
.enemy-card.avoid .portrait { filter: brightness(0.72) contrast(1.1) saturate(0.85); }

.vignette {
  position: absolute; inset: 0; pointer-events: none;
}
.ally-vignette {
  background: linear-gradient(to bottom, transparent 55%, rgba(0,0,40,0.8) 100%);
}
.enemy-vignette {
  background: linear-gradient(to bottom, transparent 45%, rgba(0,0,0,0.88) 100%),
              linear-gradient(to right, rgba(0,0,0,0.2) 0%, transparent 20%, transparent 80%, rgba(0,0,0,0.2) 100%);
  border-bottom: 2px solid var(--vc);
}

.role-tag {
  position: absolute; top: 3px; left: 50%; transform: translateX(-50%);
  font-size: 6.5px; font-weight: 800; letter-spacing: 0.7px;
  color: rgba(240,230,210,0.88); background: rgba(0,0,0,0.55);
  padding: 1px 5px; border-radius: 2px; text-transform: uppercase;
  backdrop-filter: blur(2px);
}

.self-crown {
  position: absolute; bottom: 5px; left: 50%; transform: translateX(-50%);
  font-size: 6px; font-weight: 900; letter-spacing: 0.5px;
  color: #c8aa6e; background: rgba(0,0,0,0.7);
  padding: 1px 5px; border-radius: 2px; white-space: nowrap;
  text-transform: uppercase;
}

/* ── ORB ────────────────────────────────── */
.orb {
  width: 88%;
  min-height: 46px;
  border-radius: 23px;
  background: var(--vb);
  border: 1.5px solid var(--vc);
  box-shadow: 0 0 14px var(--vg), inset 0 0 10px rgba(0,0,0,0.65);
  display: flex; flex-direction: column;
  align-items: center; justify-content: center;
  gap: 1px;
  padding: 5px 8px;
  margin-top: -20px; z-index: 2; position: relative;
  flex-shrink: 0; backdrop-filter: blur(4px);
}
.orb-icon {
  font-size: 11px; font-weight: 900;
  color: var(--vc); line-height: 1; font-family: monospace;
}
.orb-label {
  font-size: 8px; font-weight: 800;
  color: var(--vc); text-transform: uppercase;
  letter-spacing: 0.5px; text-align: center; line-height: 1.2;
}
.orb-wr {
  font-size: 7px; font-weight: 700;
  color: var(--vc); opacity: 0.75; line-height: 1;
}

/* ── NAMES ──────────────────────────────── */
.card-name {
  font-size: 8.5px; font-weight: 700; color: #f0e6d2;
  text-align: center; white-space: nowrap;
  overflow: hidden; text-overflow: ellipsis; width: 100%;
}

/* ── VS DIVIDER ─────────────────────────── */
.vs-divider {
  display: flex; align-items: center; gap: 8px;
  padding: 5px 14px; flex-shrink: 0;
}
.vs-line { flex: 1; height: 1px; background: rgba(200,170,110,0.15); }
.vs-text {
  font-size: 9px; font-weight: 900; letter-spacing: 2px;
  color: rgba(200,170,110,0.35);
}

/* ── AVOID BANNER ───────────────────────── */
.avoid-banner {
  display: flex; align-items: center; gap: 7px;
  margin: 6px 10px 0;
  padding: 5px 10px;
  background: rgba(255,78,78,0.07);
  border: 1px solid rgba(255,78,78,0.2);
  border-radius: 4px;
  font-size: 10px; color: #f0e6d2;
  flex-shrink: 0;
}
.avoid-icon { font-size: 11px; color: #ff7070; flex-shrink: 0; }
.avoid-banner strong { color: #ff7070; }

/* ── FOOTER ─────────────────────────────── */
.footer {
  display: flex; align-items: center; justify-content: space-between;
  padding: 6px 12px 7px; margin-top: 6px;
  border-top: 1px solid rgba(200,170,110,0.1);
  flex-shrink: 0;
}
.brand {
  font-size: 6.5px; font-weight: 800; letter-spacing: 1.5px;
  color: #3a3830; text-transform: uppercase;
}
.esc-btn {
  background: rgba(255,255,255,0.04); border: 1px solid rgba(255,255,255,0.08);
  border-radius: 3px; color: #5b5a56; font-size: 7.5px; font-weight: 700;
  padding: 2px 8px; cursor: pointer; letter-spacing: 0.5px; transition: all 0.15s;
}
.esc-btn:hover { background: rgba(255,78,78,0.12); border-color: rgba(255,78,78,0.35); color: #ff7070; }
</style>
