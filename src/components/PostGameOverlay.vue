<script setup lang="ts">
import { computed } from 'vue';

interface MetricGrade {
  label: string;
  player_value: number;
  benchmark_value: number;
  unit: string;
  grade: string;
  feedback: string;
}

interface PostGameReport {
  champion: string;
  role: string;
  win: boolean;
  duration_min: number;
  metrics: MetricGrade[];
  overall_grade: string;
  priority_tip: string;
}

const props = defineProps<{ report: PostGameReport }>();
const emit = defineEmits<{ close: [] }>();

const gradeColor = (g: string): string => {
  if (g === 'S') return '#00ff9d';
  if (g === 'A') return '#00e5ff';
  if (g === 'B') return '#4fc3f7';
  if (g === 'C') return '#ffb74d';
  return '#ef5350';
};

const fmt = (v: number, unit: string): string =>
  unit === 'mortes' ? String(Math.round(v)) : v.toFixed(1);

const durStr = computed(() => {
  const m = Math.floor(props.report.duration_min);
  const s = Math.round((props.report.duration_min % 1) * 60);
  return `${m}:${String(s).padStart(2, '0')}`;
});

const roleLabel = (r: string): string =>
  ({ TOP: 'Topo', JUNGLE: 'Selva', MID: 'Meio', ADC: 'Atirador', SUPPORT: 'Suporte' }[r] ?? r);

const weakMetrics = computed(() =>
  props.report.metrics.filter(m => ['D', 'C'].includes(m.grade))
);
const strongMetrics = computed(() =>
  props.report.metrics.filter(m => ['S', 'A', 'B'].includes(m.grade))
);

const metricContext = (m: MetricGrade): string => {
  const higher = m.unit !== 'mortes';
  const better = higher ? m.player_value >= m.benchmark_value : m.player_value <= m.benchmark_value;
  const diff = Math.abs(m.player_value - m.benchmark_value).toFixed(1);
  const sign = better ? '+' : '-';
  return `${fmt(m.player_value, m.unit)} ${m.unit}  ${sign}${diff} vs ref (${fmt(m.benchmark_value, m.unit)})`;
};

const cardLabel = (m: MetricGrade): string => {
  if (m.grade === 'S') return 'Excepcional';
  if (m.grade === 'A') return 'Acima da média';
  if (m.grade === 'B') return 'Na média';
  if (m.grade === 'C') return 'Abaixo da média';
  return 'Precisa melhorar';
};
</script>

<template>
  <div class="pg-root">

    <!-- Top bar (window chrome) -->
    <div class="top-bar" data-tauri-drag-region>
      <div class="top-left">
        <div class="macos-dots">
          <span class="dot dot-red" @click="emit('close')"></span>
          <span class="dot dot-yellow"></span>
          <span class="dot dot-green"></span>
        </div>
        <span class="brand">RIFTSYNC_CLIENT_V1.0.2</span>
      </div>
      <span :class="['result-badge', report.win ? 'win' : 'loss']">
        {{ report.win ? 'VITÓRIA' : 'DERROTA' }}
      </span>
    </div>

    <!-- Body -->
    <div class="body">

      <!-- Left sidebar -->
      <div class="left-col">
        <!-- Champion card -->
        <div class="champ-card">
          <div class="champ-avatar">{{ report.champion.slice(0, 2).toUpperCase() }}</div>
          <div class="champ-meta">
            <span class="champ-name">{{ report.champion }}</span>
            <span class="champ-role">{{ roleLabel(report.role) }}</span>
            <span class="champ-dur">{{ durStr }}</span>
          </div>
        </div>

        <!-- Divider -->
        <div class="divider" />

        <!-- Overall grade -->
        <div class="grade-section">
          <span class="grade-sub-label">NOTA GERAL</span>
          <div class="grade-big"
            :style="{ color: gradeColor(report.overall_grade), boxShadow: `0 0 28px ${gradeColor(report.overall_grade)}44`, borderColor: gradeColor(report.overall_grade) }">
            {{ report.overall_grade }}
          </div>
        </div>

        <!-- Divider -->
        <div class="divider" />

        <!-- Metric summary list -->
        <div class="metric-list">
          <div v-for="m in report.metrics" :key="m.label" class="ml-item">
            <span class="ml-grade"
              :style="{ color: gradeColor(m.grade), borderColor: gradeColor(m.grade) }">
              {{ m.grade }}
            </span>
            <div class="ml-bar-col">
              <div class="ml-header">
                <span class="ml-name">{{ m.label.toUpperCase() }}</span>
                <span class="ml-val" :style="{ color: gradeColor(m.grade) }">
                  {{ fmt(m.player_value, m.unit) }}
                  <span class="ml-bench">/ {{ fmt(m.benchmark_value, m.unit) }}</span>
                </span>
              </div>
              <div class="ml-track">
                <div class="ml-fill"
                  :style="{
                    width: Math.min(
                      (m.unit !== 'mortes'
                        ? m.player_value / Math.max(m.benchmark_value, 0.01)
                        : m.benchmark_value / Math.max(m.player_value, 0.01)) * 100,
                      120
                    ) + '%',
                    background: gradeColor(m.grade)
                  }" />
              </div>
            </div>
          </div>
        </div>
      </div>

      <!-- Right main area -->
      <div class="right-col">

        <!-- 4 Metric stat cards -->
        <div class="stat-cards">
          <div v-for="m in report.metrics" :key="m.label" class="stat-card">
            <span class="sc-name">{{ m.label.toUpperCase() }}</span>
            <span class="sc-value" :style="{ color: gradeColor(m.grade) }">
              {{ fmt(m.player_value, m.unit) }}
            </span>
            <span class="sc-label" :style="{ color: gradeColor(m.grade) }">
              {{ cardLabel(m) }}
            </span>
          </div>
        </div>

        <!-- AI coaching callout -->
        <div class="ai-callout">
          <div class="callout-top">
            <span class="callout-dot" />
            <span class="callout-label">RIFT SYNC AI — ANÁLISE DA PARTIDA</span>
          </div>
          <p class="callout-text">"{{ report.priority_tip }}"</p>
        </div>

        <!-- Feedback sections -->
        <div class="feedback-area">

          <div v-if="weakMetrics.length" class="fb-section">
            <div class="fb-section-title weak">▼ O QUE MELHORAR</div>
            <div v-for="m in weakMetrics" :key="m.label" class="fb-item">
              <span class="fb-grade"
                :style="{ color: gradeColor(m.grade), borderColor: gradeColor(m.grade) }">
                {{ m.grade }}
              </span>
              <div class="fb-text">
                <span class="fb-metric">{{ m.label }}</span>
                <span class="fb-context">{{ metricContext(m) }}</span>
                <span class="fb-action">{{ m.feedback }}</span>
              </div>
            </div>
          </div>

          <div v-if="strongMetrics.length" class="fb-section">
            <div class="fb-section-title strong">▲ PONTOS FORTES</div>
            <div v-for="m in strongMetrics" :key="m.label" class="fb-item">
              <span class="fb-grade"
                :style="{ color: gradeColor(m.grade), borderColor: gradeColor(m.grade) }">
                {{ m.grade }}
              </span>
              <div class="fb-text">
                <span class="fb-metric">{{ m.label }}</span>
                <span class="fb-context">{{ metricContext(m) }}</span>
                <span class="fb-action">{{ m.feedback }}</span>
              </div>
            </div>
          </div>

        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
* { box-sizing: border-box; }

.pg-root {
  width: 720px;
  height: 500px;
  background: #050e1a;
  border: 1px solid #102030;
  border-radius: 8px;
  font-family: 'Segoe UI', monospace;
  color: #c8d8e8;
  display: flex;
  flex-direction: column;
  overflow: hidden;
}

/* ── Top bar ── */
.top-bar {
  height: 40px;
  flex-shrink: 0;
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 0 16px;
  background: #060f1b;
  border-bottom: 1px solid #102030;
}
.top-left { display: flex; align-items: center; gap: 12px; }
.macos-dots { display: flex; gap: 7px; }
.dot { width: 12px; height: 12px; border-radius: 50%; }
.dot-red  { background: #ff5f57; cursor: pointer; }
.dot-yellow { background: #febc2e; }
.dot-green  { background: #28c840; }
.brand { font-size: 9px; font-weight: 700; color: #3a6a7a; letter-spacing: 1.5px; }

.result-badge {
  font-size: 9px; font-weight: 900; letter-spacing: 1.5px;
  padding: 4px 12px; border-radius: 3px;
}
.result-badge.win  { background: rgba(0,255,157,0.08); border: 1px solid rgba(0,255,157,0.3); color: #00ff9d; }
.result-badge.loss { background: rgba(239,83,80,0.08); border: 1px solid rgba(239,83,80,0.3); color: #ef5350; }

/* ── Body ── */
.body {
  display: flex;
  flex: 1;
  overflow: hidden;
}

/* ── Left col ── */
.left-col {
  width: 200px;
  flex-shrink: 0;
  border-right: 1px solid #102030;
  display: flex;
  flex-direction: column;
  padding: 12px;
  gap: 10px;
  overflow: hidden;
}

.champ-card {
  display: flex;
  align-items: center;
  gap: 10px;
}
.champ-avatar {
  width: 44px; height: 44px;
  background: rgba(0,229,255,0.06);
  border: 1px solid rgba(0,229,255,0.2);
  border-radius: 6px;
  display: flex; align-items: center; justify-content: center;
  font-size: 14px; font-weight: 900; color: #00e5ff;
  flex-shrink: 0;
}
.champ-meta { display: flex; flex-direction: column; gap: 2px; min-width: 0; }
.champ-name { font-size: 14px; font-weight: 800; color: #e8f4ff; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
.champ-role { font-size: 8px; font-weight: 700; color: #00e5ff; letter-spacing: 0.5px; }
.champ-dur  { font-size: 9px; color: #3a5a6a; }

.divider { height: 1px; background: #102030; flex-shrink: 0; }

.grade-section { display: flex; flex-direction: column; align-items: center; gap: 6px; padding: 4px 0; flex-shrink: 0; }
.grade-sub-label { font-size: 7px; font-weight: 700; color: #3a6a7a; letter-spacing: 1.5px; }
.grade-big {
  width: 56px; height: 56px;
  border: 2px solid;
  border-radius: 8px;
  display: flex; align-items: center; justify-content: center;
  font-size: 30px; font-weight: 900; line-height: 1;
}

.metric-list { display: flex; flex-direction: column; gap: 8px; flex: 1; overflow: hidden; }
.ml-item { display: flex; align-items: center; gap: 6px; }
.ml-grade {
  font-size: 9px; font-weight: 900;
  width: 20px; height: 20px;
  border: 1px solid;
  border-radius: 3px;
  display: flex; align-items: center; justify-content: center;
  flex-shrink: 0;
}
.ml-bar-col { flex: 1; display: flex; flex-direction: column; gap: 3px; min-width: 0; }
.ml-header { display: flex; justify-content: space-between; align-items: baseline; }
.ml-name  { font-size: 8px; font-weight: 700; color: #3a6a7a; letter-spacing: 0.5px; white-space: nowrap; }
.ml-val   { font-size: 9px; font-weight: 700; }
.ml-bench { font-size: 8px; color: #1a4a5a; font-weight: 400; }
.ml-track { height: 3px; background: #0a1a2a; border-radius: 2px; overflow: hidden; }
.ml-fill  { height: 100%; border-radius: 2px; transition: width 0.9s cubic-bezier(0.23,1,0.32,1); }

/* ── Right col ── */
.right-col {
  flex: 1;
  display: flex;
  flex-direction: column;
  padding: 12px;
  gap: 10px;
  overflow: hidden;
}

/* Stat cards row */
.stat-cards {
  display: grid;
  grid-template-columns: repeat(4, 1fr);
  gap: 8px;
  flex-shrink: 0;
}
.stat-card {
  background: #060f1b;
  border: 1px solid #102030;
  border-radius: 6px;
  padding: 10px 10px 8px;
  display: flex;
  flex-direction: column;
  gap: 4px;
}
.sc-name  { font-size: 7px; font-weight: 700; color: #3a6a7a; letter-spacing: 1px; }
.sc-value { font-size: 20px; font-weight: 900; line-height: 1; }
.sc-label { font-size: 8px; font-weight: 600; }

/* AI callout */
.ai-callout {
  flex-shrink: 0;
  padding: 11px 14px;
  background: rgba(0,229,255,0.03);
  border: 1px solid rgba(0,229,255,0.15);
  border-radius: 6px;
}
.callout-top { display: flex; align-items: center; gap: 8px; margin-bottom: 5px; }
.callout-dot {
  width: 7px; height: 7px; border-radius: 50%;
  background: #00e5ff;
  box-shadow: 0 0 10px rgba(0,229,255,0.8);
  animation: blink 2s ease-in-out infinite;
  flex-shrink: 0;
}
.callout-label { font-size: 8px; font-weight: 800; color: #00e5ff; letter-spacing: 2px; }
.callout-text  { font-size: 11px; color: #7a9ab4; line-height: 1.5; font-style: italic; margin: 0; }

/* Feedback area */
.feedback-area {
  flex: 1;
  display: flex;
  flex-direction: column;
  gap: 8px;
  overflow-y: auto;
}
.feedback-area::-webkit-scrollbar { width: 3px; }
.feedback-area::-webkit-scrollbar-track { background: transparent; }
.feedback-area::-webkit-scrollbar-thumb { background: #102030; border-radius: 2px; }

.fb-section { display: flex; flex-direction: column; gap: 6px; }
.fb-section-title {
  font-size: 8px; font-weight: 800; letter-spacing: 1.5px;
  padding-bottom: 5px;
  border-bottom: 1px solid #102030;
}
.fb-section-title.weak   { color: #ef5350; }
.fb-section-title.strong { color: #00ff9d; }

.fb-item { display: flex; align-items: flex-start; gap: 8px; }
.fb-grade {
  font-size: 9px; font-weight: 900;
  width: 20px; height: 20px;
  border: 1px solid;
  border-radius: 3px;
  display: flex; align-items: center; justify-content: center;
  flex-shrink: 0; margin-top: 2px;
}
.fb-text { display: flex; flex-direction: column; gap: 2px; min-width: 0; }
.fb-metric  { font-size: 10px; font-weight: 700; color: #c8d8e8; }
.fb-context { font-size: 9px; color: #00e5ff; opacity: 0.6; font-variant-numeric: tabular-nums; }
.fb-action  { font-size: 9px; color: #5a7a8a; line-height: 1.45; }

@keyframes blink {
  0%, 100% { opacity: 1; }
  50%       { opacity: 0.3; }
}
</style>
