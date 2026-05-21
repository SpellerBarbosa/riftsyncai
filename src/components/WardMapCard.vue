<script setup lang="ts">
import { computed } from "vue";

interface WardPoint {
  x: number;   // game coordinate 0–15000
  y: number;   // game coordinate 0–15000
  priority: number; // 1 (alta) – 5 (baixa)
}

const props = defineProps<{
  champion: string;
  role: string;
  phase: string;       // "early" | "mid" | "late"
  teamSide: string;    // "blue" | "red"
  wards: WardPoint[];
  gameTime: number;
  objective?: string;       // "Dragão", "Barão", "Arauto", "Aronguejo"
  objectiveEmoji?: string;  // "🐉", "💜", "🔮", "🦀"
  secondsToSpawn?: number;  // segundos restantes para o spawn
}>();

// Posição SVG aproximada de cada objetivo no mapa (para pulse visual)
const objectiveSvgPos = computed(() => {
  switch (props.objective) {
    case "Dragão":     return { x: 178, y: 200 };
    case "Barão":      return { x: 60,  y: 44  };
    case "Arauto":     return { x: 60,  y: 44  };
    case "Aronguejo": return { x: 148, y: 148 };
    default:           return null;
  }
});

// LoL map: coordinates 0–15000 → SVG 240×240
const MAP_SIZE = 15000;
const SVG_SIZE = 240;

function toSvg(gameCoord: number, axis: "x" | "y"): number {
  const ratio = gameCoord / MAP_SIZE;
  // Y axis: game 0 = bottom, SVG 0 = top → flip
  return axis === "x"
    ? ratio * SVG_SIZE
    : (1 - ratio) * SVG_SIZE;
}

// Cores por prioridade
const priorityColor = (p: number): string => {
  if (p <= 1) return "#f0e84a";  // dourado — alta prioridade
  if (p <= 2) return "#4af0a0";  // verde
  if (p <= 3) return "#4ab4f0";  // azul
  return "#a0a0a0";              // cinza — baixa prioridade
};

const priorityLabel = (p: number): string => {
  if (p <= 1) return "★";
  if (p <= 2) return "▲";
  return "●";
};

const phaseLabel = computed(() => {
  if (props.phase === "early") return "Early (0–10min)";
  if (props.phase === "mid")   return "Mid (10–20min)";
  return "Late (20min+)";
});

const roleLabel = computed(() => {
  const map: Record<string, string> = {
    TOP: "Topo", JUNGLE: "Selva", MID: "Meio",
    ADC: "Atirador", SUPPORT: "Suporte",
  };
  return map[props.role] ?? props.role;
});

const minuteStr = computed(() => {
  const m = Math.floor(props.gameTime / 60);
  const s = Math.floor(props.gameTime % 60);
  return `${m}:${String(s).padStart(2, "0")}`;
});
</script>

<template>
  <div class="ward-card">
    <div class="ward-header" :class="{ 'ward-header--objective': objective }">
      <span class="ward-title">
        <template v-if="objective">
          {{ objectiveEmoji }} {{ objective }} em ~{{ secondsToSpawn }}s — Visão
        </template>
        <template v-else>
          👁️ Wards — {{ roleLabel }}
        </template>
      </span>
      <span class="ward-phase">{{ phaseLabel }} · {{ minuteStr }}</span>
    </div>

    <div class="map-wrapper">
      <!-- Minimap SVG -->
      <svg
        :width="SVG_SIZE"
        :height="SVG_SIZE"
        viewBox="0 0 240 240"
        class="minimap"
      >
        <!-- Fundo do mapa -->
        <rect width="240" height="240" rx="6" fill="#0d1b0f" />

        <!-- Terrain básico -->
        <!-- Rio diagonal -->
        <polygon
          points="0,160 80,240 160,240 240,80 240,160 160,240 80,240 0,80"
          fill="none"
        />
        <!-- Rio área -->
        <path
          d="M 0 170 L 70 240 L 170 240 L 240 70 L 240 30 L 200 30 L 50 0 L 0 80 Z"
          fill="none"
          stroke="#1a3a2a"
          stroke-width="18"
          opacity="0.6"
        />
        <!-- Borda do mapa -->
        <rect width="240" height="240" rx="6" fill="none" stroke="#2a4a2a" stroke-width="2" />

        <!-- Jungles (zonas de mato) -->
        <rect x="2" y="2" width="100" height="100" rx="4" fill="#132a13" opacity="0.5" />
        <rect x="138" y="138" width="100" height="100" rx="4" fill="#2a1313" opacity="0.5" />

        <!-- Torres aproximadas (azul = baixo-esquerda, vermelho = cima-direita) -->
        <!-- Base Blue -->
        <rect x="4" y="198" width="28" height="28" rx="4"
              :fill="teamSide === 'blue' ? '#1a4adf' : '#df1a1a'"
              opacity="0.8" />
        <text x="18" y="216" text-anchor="middle" font-size="9"
              fill="white" font-weight="bold">
          {{ teamSide === "blue" ? "B" : "R" }}
        </text>

        <!-- Base Red -->
        <rect x="208" y="10" width="28" height="28" rx="4"
              :fill="teamSide === 'blue' ? '#df1a1a' : '#1a4adf'"
              opacity="0.8" />
        <text x="222" y="28" text-anchor="middle" font-size="9"
              fill="white" font-weight="bold">
          {{ teamSide === "blue" ? "R" : "B" }}
        </text>

        <!-- Dragão (bottom-right área) -->
        <text x="178" y="205" text-anchor="middle" font-size="14"
              :opacity="objective === 'Dragão' ? 1.0 : 0.5">🐉</text>
        <!-- Barão / Arauto (top-left área) -->
        <text x="62" y="42" text-anchor="middle" font-size="14"
              :opacity="(objective === 'Barão' || objective === 'Arauto') ? 1.0 : 0.5">
          {{ (objective === 'Barão' || objective === 'Arauto') ? objectiveEmoji : '💜' }}
        </text>

        <!-- Pulso do objetivo ativo -->
        <g v-if="objectiveSvgPos">
          <circle
            :cx="objectiveSvgPos.x" :cy="objectiveSvgPos.y"
            r="18" fill="none" stroke="#f0e84a" stroke-width="2"
            class="obj-pulse"
          />
          <circle
            :cx="objectiveSvgPos.x" :cy="objectiveSvgPos.y"
            r="24" fill="none" stroke="#f0e84a" stroke-width="1"
            class="obj-pulse obj-pulse--slow"
            opacity="0.5"
          />
        </g>

        <!-- Ward points -->
        <g v-for="(w, i) in wards" :key="i">
          <!-- Halo de destaque -->
          <circle
            :cx="toSvg(w.x, 'x')"
            :cy="toSvg(w.y, 'y')"
            r="11"
            :fill="priorityColor(w.priority)"
            opacity="0.15"
          />
          <!-- Círculo principal -->
          <circle
            :cx="toSvg(w.x, 'x')"
            :cy="toSvg(w.y, 'y')"
            r="7"
            :fill="priorityColor(w.priority)"
            stroke="#000"
            stroke-width="1.5"
            opacity="0.95"
          />
          <!-- Número do ponto -->
          <text
            :x="toSvg(w.x, 'x')"
            :y="toSvg(w.y, 'y') + 4"
            text-anchor="middle"
            font-size="7"
            font-weight="bold"
            fill="#000"
          >{{ i + 1 }}</text>
        </g>
      </svg>

      <!-- Legenda lateral -->
      <div class="ward-legend">
        <div
          v-for="(w, i) in wards"
          :key="i"
          class="legend-item"
        >
          <span class="legend-dot" :style="{ background: priorityColor(w.priority) }">
            {{ i + 1 }}
          </span>
          <span class="legend-text">
            {{ priorityLabel(w.priority) }}
            <small>{{ Math.round(w.x / 100) * 100 }},{{ Math.round(w.y / 100) * 100 }}</small>
          </span>
        </div>
        <div v-if="!wards.length" class="no-wards">
          Sem dados de ward para {{ roleLabel }}
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.ward-card {
  width: 340px;
  background: rgba(1, 10, 19, 0.97);
  border: 1px solid #2a4a2a;
  border-radius: 8px;
  padding: 8px;
  font-family: monospace;
}

.ward-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 6px;
}

.ward-title {
  font-size: 11px;
  font-weight: 800;
  color: #4af0a0;
  letter-spacing: 0.5px;
}

.ward-phase {
  font-size: 9px;
  color: #a09b8c;
  font-weight: 600;
}

.map-wrapper {
  display: flex;
  gap: 8px;
  align-items: flex-start;
}

.minimap {
  border-radius: 6px;
  border: 1px solid #2a4a2a;
  flex-shrink: 0;
}

.ward-legend {
  display: flex;
  flex-direction: column;
  gap: 4px;
  flex: 1;
  max-height: 240px;
  overflow-y: auto;
}

.legend-item {
  display: flex;
  align-items: center;
  gap: 5px;
}

.legend-dot {
  width: 18px;
  height: 18px;
  border-radius: 50%;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 8px;
  font-weight: 800;
  color: #000;
  flex-shrink: 0;
  border: 1px solid rgba(0,0,0,0.5);
}

.legend-text {
  font-size: 8px;
  color: #c8aa6e;
  line-height: 1.3;
}

.legend-text small {
  display: block;
  color: #5a5a5a;
  font-size: 7px;
}

.no-wards {
  font-size: 9px;
  color: #666;
  padding: 4px;
}

/* Header contextual quando há objetivo */
.ward-header--objective .ward-title {
  color: #f0e84a;
  font-size: 10px;
}

/* Pulso animado no objetivo */
@keyframes obj-pulse {
  0%   { r: 14; opacity: 0.9; }
  70%  { r: 22; opacity: 0.2; }
  100% { r: 14; opacity: 0;   }
}
@keyframes obj-pulse-slow {
  0%   { r: 20; opacity: 0.5; }
  70%  { r: 30; opacity: 0.1; }
  100% { r: 20; opacity: 0;   }
}
.obj-pulse {
  animation: obj-pulse 1.4s ease-out infinite;
}
.obj-pulse--slow {
  animation: obj-pulse-slow 1.4s ease-out 0.4s infinite;
}
</style>
