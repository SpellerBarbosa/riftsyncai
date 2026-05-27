<script setup lang="ts">
import { computed } from "vue";
import { getCurrentWindow } from "@tauri-apps/api/window";

const hideWindow = () => getCurrentWindow().hide();

interface WardPoint { x: number; y: number; priority: number; ward_type: string; }

const props = defineProps<{
  champion: string; role: string; phase: string; teamSide: string;
  wards: WardPoint[]; gameTime: number; objective?: string;
  objectiveEmoji?: string; secondsToSpawn?: number;
}>();

const MAP_PX = 240;
const GAME_MAX = 15000;

function toMap(gameCoord: number, axis: "x" | "y"): number {
  const ratio = gameCoord / GAME_MAX;
  return axis === "x" ? ratio * MAP_PX : (1 - ratio) * MAP_PX;
}

const objectiveGamePos = computed(() => {
  switch (props.objective) {
    case "Dragão":    return { x: 9866,  y: 4414  };
    case "Barão":     return { x: 4951,  y: 10440 };
    case "Arauto":    return { x: 4951,  y: 10440 };
    case "Aronguejo": return { x: 10991, y: 5009  };
    default: return null;
  }
});

const wardColor = (w: WardPoint) => {
  if (w.ward_type === "pink") return "#ff69b4";
  if (w.priority <= 1) return "#f0e84a";
  if (w.priority <= 2) return "#4af076";
  if (w.priority <= 3) return "#4ab4f0";
  return "#c0c0c0";
};

const roleLabel = computed(() => {
  const m: Record<string, string> = { TOP: "Topo", JUNGLE: "Selva", MID: "Meio", ADC: "Atirador", SUPPORT: "Suporte" };
  return m[props.role] ?? props.role;
});

const minuteStr = computed(() => {
  const m = Math.floor(props.gameTime / 60);
  const s = Math.floor(props.gameTime % 60);
  return `${m}:${String(s).padStart(2, "0")}`;
});

const topWards = computed(() => {
  const limit = props.role?.toUpperCase() === "SUPPORT" ? 4 : 3;
  return props.wards.slice(0, limit);
});

const urgencyPct = computed(() => {
  if (props.secondsToSpawn == null) return 0;
  return Math.round(((60 - Math.max(0, Math.min(60, props.secondsToSpawn))) / 60) * 100);
});

const urgencyColor = computed(() => {
  const p = urgencyPct.value;
  if (p >= 80) return "#ff4e4e";
  if (p >= 50) return "#f0a84a";
  return "#4af0a0";
});

const spawningSoon = computed(() => (props.secondsToSpawn ?? 99) <= 10);

function wardDesc(w: WardPoint, index: number, objective: string | undefined): string {
  if (w.ward_type === "pink") return "🔮 Ward de Controle — visão permanente";
  if (objective) {
    const isRed = props.teamSide === "red";
    const descs: Record<string, string[]> = {
      "Dragão":    isRed ? ["Entrada aliada (sul)", "Entrada inimiga (norte)"] : ["Entrada aliada (norte)", "Entrada inimiga (sul)"],
      "Barão":     isRed ? ["Entrada aliada (leste)", "Tri-bush inimigo (oeste)"] : ["Tri-bush aliado (oeste)", "Entrada inimiga (leste)"],
      "Arauto":    isRed ? ["Entrada aliada (leste)", "Tri-bush inimigo (oeste)"] : ["Tri-bush aliado (oeste)", "Entrada inimiga (leste)"],
      "Aronguejo": ["Arbusto rio bot", "Arbusto rio top"],
    };
    return (descs[objective] ?? [])[index] ?? `Ponto ${index + 1}`;
  }
  return index === 0 ? "★ Prioridade máxima" : "▲ Visão crítica";
}
</script>

<template>
  <div class="w-65 bg-[rgba(1,8,16,0.97)] border rounded-lg p-1.5 font-mono select-none transition-colors duration-300"
       :class="spawningSoon ? 'border-[rgba(255,78,78,0.8)] shadow-[0_0_10px_rgba(255,78,78,0.3)]' : 'border-[rgba(200,170,110,0.5)]'">

    <!-- Header -->
    <div class="flex justify-between items-center mb-1 px-0.5">
      <span class="text-[10px] font-bold tracking-[0.3px]"
            :class="objective ? 'text-[#f0e84a]' : 'text-[#4af0a0]'">
        <template v-if="objective">{{ objectiveEmoji }} <strong>{{ objective }}</strong></template>
        <template v-else>👁️ Visão · {{ roleLabel }}</template>
      </span>
      <div class="flex items-center gap-1.5">
        <span class="text-[9px] text-[#a09b8c] font-semibold">{{ minuteStr }}</span>
        <button class="bg-transparent border-none text-[#5b5a56] text-sm cursor-pointer px-0.5 leading-none rounded-sm transition-colors hover:text-[#ff4e4e]"
                @click="hideWindow" title="Fechar">×</button>
      </div>
    </div>

    <!-- Countdown -->
    <div v-if="objective && secondsToSpawn != null" class="mb-1">
      <div class="flex items-center gap-1 border rounded px-1.5 py-0.5 mb-0.5 transition-colors"
           :style="{ borderColor: urgencyColor }">
        <span class="text-[9px]">⏱</span>
        <span class="text-[9px] font-bold tracking-[0.2px] transition-colors" :style="{ color: urgencyColor }">
          <template v-if="spawningSoon">⚡ Aparecendo agora!</template>
          <template v-else>{{ secondsToSpawn }}s para spawn — Coloque visão!</template>
        </span>
      </div>
      <div class="h-0.75 bg-white/8 rounded overflow-hidden">
        <div class="h-full rounded transition-[width,background] duration-[1s,300ms] ease-linear"
             :style="{ width: urgencyPct + '%', background: urgencyColor }" />
      </div>
    </div>

    <!-- Map -->
    <div class="relative w-62 h-62 rounded overflow-hidden border border-white/10">
      <img src="/minimap.png" class="absolute inset-0 w-full h-full object-cover block" alt="minimap" draggable="false" />
      <svg class="absolute inset-0 w-full h-full pointer-events-none"
           :width="MAP_PX" :height="MAP_PX" :viewBox="`0 0 ${MAP_PX} ${MAP_PX}`">
        <g v-if="objectiveGamePos">
          <circle :cx="toMap(objectiveGamePos.x,'x')" :cy="toMap(objectiveGamePos.y,'y')" r="16" fill="none" stroke="#f0e84a" stroke-width="2.5" class="obj-pulse" />
          <circle :cx="toMap(objectiveGamePos.x,'x')" :cy="toMap(objectiveGamePos.y,'y')" r="22" fill="none" stroke="#f0e84a" stroke-width="1" class="obj-pulse obj-pulse--slow" opacity="0.4" />
          <text :x="toMap(objectiveGamePos.x,'x')" :y="toMap(objectiveGamePos.y,'y')+4" text-anchor="middle" font-size="10">{{ objectiveEmoji }}</text>
        </g>
        <g v-for="(w, i) in topWards" :key="i">
          <circle :cx="toMap(w.x,'x')" :cy="toMap(w.y,'y')" r="10" :fill="wardColor(w)" opacity="0.20" />
          <circle :cx="toMap(w.x,'x')" :cy="toMap(w.y,'y')" r="6" :fill="wardColor(w)" stroke="rgba(0,0,0,0.8)" stroke-width="1.5" />
          <circle v-if="w.ward_type==='pink'" :cx="toMap(w.x,'x')" :cy="toMap(w.y,'y')" r="9" fill="none" stroke="#ff69b4" stroke-width="1" opacity="0.6" />
          <text :x="toMap(w.x,'x')" :y="toMap(w.y,'y')+4" text-anchor="middle" font-size="6" font-weight="900" fill="#000">{{ i+1 }}</text>
        </g>
      </svg>
    </div>

    <!-- Legend -->
    <div class="flex flex-col gap-0.75 mt-1 px-0.5">
      <div v-if="!topWards.length" class="text-[9px] text-[#555] py-0.5 w-full text-center">
        Sem dados de ward para {{ roleLabel }}
      </div>
      <div v-for="(w, i) in topWards" :key="i" class="flex items-center gap-1">
        <span class="w-3.5 h-3.5 rounded-full flex items-center justify-center text-[7px] font-black text-black shrink-0"
              :style="{ background: wardColor(w) }">{{ i+1 }}</span>
        <span class="text-[8px]" :style="w.ward_type === 'pink' ? { color: '#ff69b4' } : { color: '#c8aa6e' }">
          {{ wardDesc(w, i, objective) }}
        </span>
      </div>
    </div>
  </div>
</template>

<style scoped>
@keyframes pulse-ring      { 0%   { r: 12; opacity: 0.9; } 70%  { r: 20; opacity: 0.1; } 100% { r: 12; opacity: 0; } }
@keyframes pulse-ring-slow { 0%   { r: 18; opacity: 0.4; } 70%  { r: 28; opacity: 0.05; } 100% { r: 18; opacity: 0; } }
.obj-pulse       { animation: pulse-ring 1.4s ease-out infinite; }
.obj-pulse--slow { animation: pulse-ring-slow 1.4s ease-out 0.4s infinite; }
</style>
