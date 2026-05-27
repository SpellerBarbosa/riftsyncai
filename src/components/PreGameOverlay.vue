<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted } from 'vue';
import { getCurrentWindow } from '@tauri-apps/api/window';

interface Ally   { name: string; champion_id: string; role: string; is_self: boolean; }
interface Matchup { enemy: string; champion_id: string; role: string; win_rate: number; verdict: 'win'|'avoid'|'even'|'unknown'; }
interface PreGameReport { player_champion: string; player_role: string; allies: Ally[]; matchups: Matchup[]; avoid_1v1: string[]; }

const appWindow = getCurrentWindow();
const report = ref<PreGameReport | null>(null);
const countdown = ref(50);
let timer: ReturnType<typeof setInterval> | null = null;

const DDRAGON_VERSION = '16.10.1';
function portraitUrl(id: string) { return `https://ddragon.leagueoflegends.com/cdn/img/champion/loading/${id}_0.jpg`; }
function iconUrl(id: string)     { return `https://ddragon.leagueoflegends.com/cdn/${DDRAGON_VERSION}/img/champion/${id}.png`; }
function onImgError(e: Event, id: string) { (e.target as HTMLImageElement).src = iconUrl(id); }

const roleLabel: Record<string, string> = { TOP:'Top', JUNGLE:'JG', MIDDLE:'Mid', BOTTOM:'ADC', UTILITY:'Sup' };
const displayRole = computed(() => {
  if (!report.value) return '';
  return roleLabel[report.value.player_role.toUpperCase()] ?? report.value.player_role;
});

const vc = {
  win:     { icon:'✓', label:'Favorável',    color:'#4eff9b', glow:'rgba(78,255,155,0.55)',  bg:'rgba(78,255,155,0.13)'  },
  avoid:   { icon:'!', label:'Evite 1v1',    color:'#ff4e4e', glow:'rgba(255,78,78,0.55)',   bg:'rgba(255,78,78,0.16)'   },
  even:    { icon:'~', label:'Equilibrado',  color:'#c8aa6e', glow:'rgba(200,170,110,0.45)', bg:'rgba(200,170,110,0.11)' },
  unknown: { icon:'?', label:'Sem dados',    color:'#5b5a56', glow:'rgba(91,90,86,0.25)',    bg:'rgba(30,30,30,0.5)'    },
} as const;

function vconf(v: string)          { return vc[v as keyof typeof vc] ?? vc.unknown; }
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
onUnmounted(() => { if (timer) clearInterval(timer); window.removeEventListener('keydown', onKey); });
</script>

<template>
  <div v-if="report"
       class="w-screen h-screen flex flex-col border border-[rgba(200,170,110,0.4)] rounded-md overflow-hidden text-[#f0e6d2] select-none shadow-[0_24px_64px_rgba(0,0,0,0.97)]"
       style="background:linear-gradient(170deg,#010810 0%,#020c16 100%);font-family:'Segoe UI',sans-serif">

    <!-- Header -->
    <div class="flex items-center justify-between shrink-0 px-3 py-1.5 border-b border-[rgba(200,170,110,0.16)] bg-[rgba(200,170,110,0.03)]"
         data-tauri-drag-region>
      <div class="flex items-center gap-1.5" data-tauri-drag-region>
        <span class="text-[6.5px] font-extrabold tracking-widest text-[#c8aa6e] bg-[rgba(200,170,110,0.1)] border border-[rgba(200,170,110,0.3)] px-1.5 py-0.5 rounded-sm">ANÁLISE DE DRAFT</span>
        <span class="text-[rgba(200,170,110,0.3)] text-[10px]">•</span>
        <span class="text-[12px] font-black text-[#f0e6d2] tracking-wide uppercase">{{ report.player_champion }}</span>
        <span class="text-[8.5px] font-bold text-[#a09b8c] bg-white/[0.06] px-1.5 py-px rounded">{{ displayRole }}</span>
      </div>
      <div class="flex items-center gap-2">
        <span class="text-[9px] font-bold text-[#5b5a56]">{{ countdown }}s</span>
        <button class="bg-none border-none text-[#a09b8c] text-lg cursor-pointer px-0.5 rounded leading-none transition-all hover:text-[#ff4e4e] hover:bg-[rgba(255,78,78,0.12)]"
                @click="close">×</button>
      </div>
    </div>

    <!-- Ally row -->
    <div class="flex items-start gap-2 px-2.5 pt-1.5 flex-1 min-h-0">
      <div class="[writing-mode:vertical-rl] [text-orientation:mixed] rotate-180 text-[6.5px] font-extrabold tracking-widest uppercase py-1 shrink-0 self-center text-[#4a9eff]/70">SEU TIME</div>
      <div class="flex gap-1 flex-1 justify-center">
        <div v-for="a in report.allies" :key="a.champion_id"
             class="flex flex-col items-center gap-0.5 flex-1 max-w-[190px]">
          <div class="relative w-full rounded overflow-hidden border"
               style="aspect-ratio:3/4"
               :class="a.is_self ? 'border-[rgba(200,170,110,0.6)] shadow-[0_0_10px_rgba(200,170,110,0.3)]' : 'border-[rgba(74,158,255,0.25)]'">
            <img :src="portraitUrl(a.champion_id)" :alt="a.name"
                 class="w-full h-full object-cover object-top block"
                 :class="a.is_self ? 'brightness-[0.92]' : 'brightness-[0.82] saturate-[0.85]'"
                 draggable="false" @error="onImgError($event, a.champion_id)" />
            <div class="absolute inset-0 pointer-events-none" style="background:linear-gradient(to bottom,transparent 55%,rgba(0,0,40,0.8) 100%)"></div>
            <span class="absolute top-0.5 left-1/2 -translate-x-1/2 text-[6.5px] font-extrabold tracking-wide text-[rgba(240,230,210,0.88)] bg-black/55 px-1 py-px rounded-sm uppercase backdrop-blur-sm">{{ a.role }}</span>
            <div v-if="a.is_self" class="absolute bottom-1 left-1/2 -translate-x-1/2 text-[6px] font-black tracking-wide text-[#c8aa6e] bg-black/70 px-1 py-px rounded-sm whitespace-nowrap uppercase">★ VOCÊ</div>
          </div>
          <span class="text-[8.5px] font-bold text-[#f0e6d2] text-center whitespace-nowrap overflow-hidden text-ellipsis w-full">{{ a.name }}</span>
        </div>
      </div>
    </div>

    <!-- VS divider -->
    <div class="flex items-center gap-2 px-3.5 py-1 shrink-0">
      <div class="flex-1 h-px bg-[rgba(200,170,110,0.15)]"></div>
      <span class="text-[9px] font-black tracking-[2px] text-[rgba(200,170,110,0.35)]">VS</span>
      <div class="flex-1 h-px bg-[rgba(200,170,110,0.15)]"></div>
    </div>

    <!-- Enemy row -->
    <div class="flex items-start gap-2 px-2.5 flex-1 min-h-0">
      <div class="[writing-mode:vertical-rl] [text-orientation:mixed] rotate-180 text-[6.5px] font-extrabold tracking-widest uppercase py-1 shrink-0 self-center text-[#ff4e4e]/70">TIME INIMIGO</div>
      <div class="flex gap-1 flex-1 justify-center">
        <div v-for="m in report.matchups" :key="m.champion_id"
             class="flex flex-col items-center gap-0.5 flex-1 max-w-[190px]">
          <div class="relative w-full rounded overflow-hidden border"
               style="aspect-ratio:3/4"
               :class="{
                 'border-[rgba(78,255,155,0.4)]': m.verdict==='win',
                 'border-[rgba(255,78,78,0.45)]':  m.verdict==='avoid',
                 'border-[rgba(200,170,110,0.3)]': m.verdict==='even',
                 'border-white/[0.07]':            m.verdict==='unknown',
               }">
            <img :src="portraitUrl(m.champion_id)" :alt="m.enemy"
                 class="w-full h-full object-cover object-top block"
                 :class="m.verdict==='avoid' ? 'brightness-[0.72] contrast-[1.1] saturate-[0.85]' : ''"
                 draggable="false" @error="onImgError($event, m.champion_id)" />
            <div class="absolute inset-0 pointer-events-none"
                 :style="`background:linear-gradient(to bottom,transparent 45%,rgba(0,0,0,0.88) 100%);border-bottom:2px solid ${vconf(m.verdict).color}`"></div>
            <span class="absolute top-0.5 left-1/2 -translate-x-1/2 text-[6.5px] font-extrabold tracking-wide text-[rgba(240,230,210,0.88)] bg-black/55 px-1 py-px rounded-sm uppercase backdrop-blur-sm">{{ m.role }}</span>
          </div>

          <!-- Orb -->
          <div class="w-[88%] min-h-[46px] rounded-[23px] border-[1.5px] flex flex-col items-center justify-center gap-px py-1 px-2 -mt-5 z-[2] relative shrink-0 backdrop-blur-[4px]"
               :style="{ background: vconf(m.verdict).bg, borderColor: vconf(m.verdict).color, boxShadow: `0 0 14px ${vconf(m.verdict).glow},inset 0 0 10px rgba(0,0,0,0.65)` }">
            <span class="text-[11px] font-black leading-none font-mono" :style="{ color: vconf(m.verdict).color }">{{ vconf(m.verdict).icon }}</span>
            <span class="text-[8px] font-extrabold uppercase tracking-wide text-center leading-tight" :style="{ color: vconf(m.verdict).color }">{{ tacticalTag(m) }}</span>
            <span class="text-[7px] font-bold leading-none opacity-75" :style="{ color: vconf(m.verdict).color }">{{ wrLabel(m.win_rate, m.verdict) }}</span>
          </div>

          <span class="text-[8.5px] font-bold text-[#f0e6d2] text-center whitespace-nowrap overflow-hidden text-ellipsis w-full">{{ m.enemy }}</span>
        </div>
      </div>
    </div>

    <!-- Avoid banner -->
    <div v-if="report.avoid_1v1.length"
         class="flex items-center gap-1.5 mx-2.5 mt-1.5 px-2.5 py-1 bg-[rgba(255,78,78,0.07)] border border-[rgba(255,78,78,0.2)] rounded text-[10px] text-[#f0e6d2] shrink-0">
      <span class="text-[11px] text-[#ff7070] shrink-0">⚠</span>
      <span>Sempre <strong class="text-[#ff7070]">2+</strong> contra <strong class="text-[#ff7070]">{{ report.avoid_1v1.join(', ') }}</strong></span>
    </div>

    <!-- Footer -->
    <div class="flex items-center justify-between px-3 py-1.5 mt-1.5 border-t border-[rgba(200,170,110,0.1)] shrink-0">
      <span class="text-[6.5px] font-extrabold tracking-widest text-[#3a3830] uppercase">SPELL COACH IA</span>
      <button class="bg-[rgba(255,255,255,0.04)] border border-[rgba(255,255,255,0.08)] rounded text-[#5b5a56] text-[7.5px] font-bold px-2 py-0.5 cursor-pointer tracking-wide transition-all hover:bg-[rgba(255,78,78,0.12)] hover:border-[rgba(255,78,78,0.35)] hover:text-[#ff7070]"
              @click="close">ESC — fechar</button>
    </div>
  </div>
</template>
