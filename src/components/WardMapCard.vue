<script setup lang="ts">
import { computed } from "vue";
import { getCurrentWindow } from "@tauri-apps/api/window";

const hideWindow = () => getCurrentWindow().hide();

interface WardPoint {
  x: number;   // game coordinate 0–15000
  y: number;   // game coordinate 0–15000
  priority: number; // 1 (alta) – 5 (baixa)
}

const props = defineProps<{
  champion: string;
  role: string;
  phase: string;
  teamSide: string;
  wards: WardPoint[];
  gameTime: number;
  objective?: string;
  objectiveEmoji?: string;
  secondsToSpawn?: number;
}>();

// Minimap do LoL: coordenadas de jogo 0–15000 → px 0–240
// Y é invertido: jogo Y=0 é bottom, imagem Y=0 é top
const MAP_PX = 240;
const GAME_MAX = 15000;

function toMap(gameCoord: number, axis: "x" | "y"): number {
  const ratio = gameCoord / GAME_MAX;
  return axis === "x" ? ratio * MAP_PX : (1 - ratio) * MAP_PX;
}

// Posição no mapa dos objetivos (coordenadas de jogo)
const objectiveGamePos = computed(() => {
  switch (props.objective) {
    case "Dragão":    return { x: 9866, y: 4414 };
    case "Barão":     return { x: 4951, y: 10440 };
    case "Arauto":    return { x: 4951, y: 10440 };
    case "Aronguejo": return { x: 10991, y: 5009 };
    default: return null;
  }
});

const priorityColor = (p: number) => {
  if (p <= 1) return "#f0e84a";
  if (p <= 2) return "#4af076";
  if (p <= 3) return "#4ab4f0";
  return "#c0c0c0";
};

const roleLabel = computed(() => {
  const m: Record<string, string> = {
    TOP: "Topo", JUNGLE: "Selva", MID: "Meio", ADC: "Atirador", SUPPORT: "Suporte",
  };
  return m[props.role] ?? props.role;
});

const minuteStr = computed(() => {
  const m = Math.floor(props.gameTime / 60);
  const s = Math.floor(props.gameTime % 60);
  return `${m}:${String(s).padStart(2, "0")}`;
});

// Suporte carrega mais visão; demais funções mostram apenas os 2 pontos mais estratégicos
const topWards = computed(() => {
  const limit = props.role?.toUpperCase() === 'SUPPORT' ? 6 : 2;
  return props.wards.slice(0, limit);
});
</script>

<template>
  <div class="ward-card">

    <!-- Cabeçalho -->
    <div class="ward-header" :class="{ 'is-objective': !!objective }">
      <span class="ward-title">
        <template v-if="objective">
          {{ objectiveEmoji }} <strong>{{ objective }}</strong> em {{ secondsToSpawn }}s — Visão
        </template>
        <template v-else>
          👁️ Wards · {{ roleLabel }}
        </template>
      </span>
      <div class="ward-header-right">
        <span class="ward-time">{{ minuteStr }}</span>
        <button class="ward-close-btn" @click="hideWindow" title="Fechar">×</button>
      </div>
    </div>

    <!-- Mapa -->
    <div class="map-container">
      <!-- Minimap real do LoL (Summoner's Rift) — bundled localmente para evitar 403 no CDN -->
      <img
        src="/minimap.png"
        class="minimap-img"
        alt="minimap"
        draggable="false"
      />

      <!-- Overlay SVG com wards e objetivos -->
      <svg
        class="ward-overlay"
        :width="MAP_PX"
        :height="MAP_PX"
        :viewBox="`0 0 ${MAP_PX} ${MAP_PX}`"
      >
        <!-- Pulso do objetivo ativo -->
        <g v-if="objectiveGamePos">
          <circle
            :cx="toMap(objectiveGamePos.x, 'x')"
            :cy="toMap(objectiveGamePos.y, 'y')"
            r="16" fill="none" stroke="#f0e84a" stroke-width="2.5"
            class="obj-pulse"
          />
          <circle
            :cx="toMap(objectiveGamePos.x, 'x')"
            :cy="toMap(objectiveGamePos.y, 'y')"
            r="22" fill="none" stroke="#f0e84a" stroke-width="1"
            class="obj-pulse obj-pulse--slow"
            opacity="0.4"
          />
        </g>

        <!-- Ward markers -->
        <g v-for="(w, i) in topWards" :key="i">
          <!-- Sombra / halo -->
          <circle
            :cx="toMap(w.x, 'x')"
            :cy="toMap(w.y, 'y')"
            r="10"
            :fill="priorityColor(w.priority)"
            opacity="0.20"
          />
          <!-- Círculo principal -->
          <circle
            :cx="toMap(w.x, 'x')"
            :cy="toMap(w.y, 'y')"
            r="6"
            :fill="priorityColor(w.priority)"
            stroke="rgba(0,0,0,0.8)"
            stroke-width="1.5"
          />
          <!-- Número -->
          <text
            :x="toMap(w.x, 'x')"
            :y="toMap(w.y, 'y') + 4"
            text-anchor="middle"
            font-size="6"
            font-weight="900"
            fill="#000"
          >{{ i + 1 }}</text>
        </g>
      </svg>
    </div>

    <!-- Legenda compacta -->
    <div class="ward-legend">
      <div v-if="!topWards.length" class="no-wards">
        Sem dados de ward para {{ roleLabel }}
      </div>
      <div
        v-for="(w, i) in topWards"
        :key="i"
        class="legend-row"
      >
        <span class="legend-num" :style="{ background: priorityColor(w.priority) }">
          {{ i + 1 }}
        </span>
        <span class="legend-desc">
          {{ i === 0 ? '★ Prioridade máxima' : i <= 2 ? '▲ Importante' : '● Situacional' }}
        </span>
      </div>
    </div>

  </div>
</template>

<style scoped>
.ward-card {
  width: 260px;
  background: rgba(1, 8, 16, 0.97);
  border: 1px solid rgba(200, 170, 110, 0.5);
  border-radius: 8px;
  padding: 6px;
  font-family: monospace;
  user-select: none;
}

/* Header */
.ward-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 5px;
  padding: 0 2px;
}
.ward-title {
  font-size: 10px;
  font-weight: 700;
  color: #4af0a0;
  letter-spacing: 0.3px;
}
.ward-header.is-objective .ward-title { color: #f0e84a; }
.ward-header-right { display: flex; align-items: center; gap: 6px; }
.ward-time { font-size: 9px; color: #a09b8c; font-weight: 600; }
.ward-close-btn {
  background: none; border: none; color: #5b5a56; font-size: 14px;
  cursor: pointer; padding: 0 2px; line-height: 1; border-radius: 2px;
  transition: color 0.15s;
}
.ward-close-btn:hover { color: #ff4e4e; }

/* Mapa */
.map-container {
  position: relative;
  width: 248px;
  height: 248px;
  border-radius: 4px;
  overflow: hidden;
  border: 1px solid rgba(255,255,255,0.1);
}
.minimap-img {
  position: absolute;
  top: 0; left: 0;
  width: 100%;
  height: 100%;
  object-fit: cover;
  display: block;
}
.ward-overlay {
  position: absolute;
  top: 0; left: 0;
  width: 100%;
  height: 100%;
  pointer-events: none;
}

/* Legenda */
.ward-legend {
  display: flex;
  flex-direction: row;
  flex-wrap: wrap;
  gap: 3px 6px;
  margin-top: 5px;
  padding: 0 2px;
}
.legend-row {
  display: flex;
  align-items: center;
  gap: 4px;
}
.legend-num {
  width: 14px;
  height: 14px;
  border-radius: 50%;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 7px;
  font-weight: 900;
  color: #000;
  flex-shrink: 0;
}
.legend-desc {
  font-size: 8px;
  color: #a09b8c;
}
.no-wards {
  font-size: 9px;
  color: #555;
  padding: 2px;
  width: 100%;
  text-align: center;
}

/* Animações de objetivo */
@keyframes pulse-ring {
  0%   { r: 12; opacity: 0.9; }
  70%  { r: 20; opacity: 0.1; }
  100% { r: 12; opacity: 0.0; }
}
@keyframes pulse-ring-slow {
  0%   { r: 18; opacity: 0.4; }
  70%  { r: 28; opacity: 0.05; }
  100% { r: 18; opacity: 0.0; }
}
.obj-pulse        { animation: pulse-ring 1.4s ease-out infinite; }
.obj-pulse--slow  { animation: pulse-ring-slow 1.4s ease-out 0.4s infinite; }
</style>
