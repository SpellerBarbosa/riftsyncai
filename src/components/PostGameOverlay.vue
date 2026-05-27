<script setup lang="ts">
import { computed } from 'vue';

interface MetricGrade {
  label: string; player_value: number; benchmark_value: number;
  unit: string; grade: string; feedback: string;
  history_avg?: number; history_note?: string;
}
interface PostGameReport {
  champion: string; role: string; win: boolean; duration_min: number;
  metrics: MetricGrade[]; overall_grade: string; priority_tip: string;
}

const props = defineProps<{ report: PostGameReport }>();
const emit  = defineEmits<{ close: [] }>();

const DDRAGON_VERSION = '16.10.1';
const champDDragonId = computed(() =>
  props.report.champion.replace(/\s*&\s*Willump/i,'').replace(/[\s'.&]/g,'')
);
const champIconUrl = computed(() =>
  `https://ddragon.leagueoflegends.com/cdn/${DDRAGON_VERSION}/img/champion/${champDDragonId.value}.png`
);

const gc = (g: string) =>
  g==='S' ? '#00ff9d' : g==='A' ? '#00e5ff' : g==='B' ? '#4fc3f7' : g==='C' ? '#ffb74d' : '#ef5350';

const fmt = (v: number, unit: string) => unit === 'mortes' ? String(Math.round(v)) : v.toFixed(1);

const durStr = computed(() => {
  const m = Math.floor(props.report.duration_min);
  const s = Math.round((props.report.duration_min % 1) * 60);
  return `${m}:${String(s).padStart(2,'0')}`;
});

const roleLabel = (r: string) =>
  ({ TOP:'Topo', JUNGLE:'Selva', MID:'Meio', ADC:'Atirador', SUPPORT:'Suporte' }[r] ?? r);

const weakMetrics   = computed(() => props.report.metrics.filter(m => ['D','C'].includes(m.grade)));
const strongMetrics = computed(() => props.report.metrics.filter(m => ['S','A','B'].includes(m.grade)));

const cardLabel = (m: MetricGrade) =>
  m.grade==='S' ? 'Excepcional' : m.grade==='A' ? 'Acima da média' :
  m.grade==='B' ? 'Na média'    : m.grade==='C' ? 'Abaixo da média' : 'Precisa melhorar';

const metricIcon = (label: string) =>
  ({ 'Mortes':'☠️','KDA':'⚔️','CS por minuto':'🌾','Visão':'👁️','Dano':'💥',
     'Dano por minuto':'💥','Ouro por minuto':'💰','Assistências':'🤝',
     'Assassinatos':'🔪','Tempo morto':'⏱️','Ward colocado':'📍','Controle de mapa':'🗺️'
  }[label] ?? '📊');

const metricContext = (m: MetricGrade) => {
  const higher = m.unit !== 'mortes';
  const diff   = Math.abs(m.player_value - m.benchmark_value).toFixed(1);
  const better = higher ? m.player_value >= m.benchmark_value : m.player_value <= m.benchmark_value;
  const ref    = `hoje: ${fmt(m.player_value,m.unit)} ${m.unit}  ${better?'+':'-'}${diff} vs ref (${fmt(m.benchmark_value,m.unit)})`;
  if (m.history_avg !== undefined) {
    const hd = Math.abs(m.player_value - m.history_avg).toFixed(1);
    const hb = higher ? m.player_value >= m.history_avg : m.player_value <= m.history_avg;
    return `${ref}  ·  ${hb ? `▲ +${hd}` : `▼ -${hd}`} vs sua média`;
  }
  return ref;
};

const fillPct = (m: MetricGrade) => Math.min(
  (m.unit !== 'mortes'
    ? m.player_value / Math.max(m.benchmark_value, 0.01)
    : m.benchmark_value / Math.max(m.player_value, 0.01)) * 100,
  120
) + '%';
</script>

<template>
  <div class="w-256 h-180 bg-[#050e1a] border border-[#102030] rounded-lg flex flex-col overflow-hidden"
       style="font-family:'Segoe UI',monospace;color:#c8d8e8">

    <!-- Top bar -->
    <div class="h-10 shrink-0 flex items-center justify-between px-4 bg-[#060f1b] border-b border-[#102030]"
         data-tauri-drag-region>
      <div class="flex items-center gap-3">
        <div class="flex gap-1.5">
          <span class="w-3 h-3 rounded-full bg-[#ff5f57] cursor-pointer" @click="emit('close')"></span>
          <span class="w-3 h-3 rounded-full bg-[#febc2e]"></span>
          <span class="w-3 h-3 rounded-full bg-[#28c840]"></span>
        </div>
        <span class="text-[11px] font-bold text-[#3a6a7a] tracking-[1.5px]">RIFTSYNC_CLIENT_V1.0.2</span>
      </div>
      <span class="text-[11px] font-black tracking-[1.5px] px-3 py-1 rounded"
            :class="report.win
              ? 'bg-[rgba(0,255,157,0.08)] border border-[rgba(0,255,157,0.3)] text-[#00ff9d]'
              : 'bg-[rgba(239,83,80,0.08)] border border-[rgba(239,83,80,0.3)] text-[#ef5350]'">
        {{ report.win ? 'VITÓRIA' : 'DERROTA' }}
      </span>
    </div>

    <!-- Body -->
    <div class="flex flex-1 overflow-hidden">

      <!-- Left sidebar -->
      <div class="w-50 shrink-0 border-r border-[#102030] flex flex-col p-3 gap-2.5 overflow-hidden">
        <!-- Champ -->
        <div class="flex items-center gap-2.5">
          <div class="w-14.5 h-14.5 border border-[rgba(0,229,255,0.25)] rounded-md overflow-hidden shrink-0 shadow-[0_0_12px_rgba(0,229,255,0.15)]">
            <img :src="champIconUrl" :alt="report.champion" class="w-full h-full object-cover block" draggable="false" />
          </div>
          <div class="flex flex-col gap-0.5 min-w-0">
            <span class="text-base font-extrabold text-[#e8f4ff] truncate">{{ report.champion }}</span>
            <span class="text-[10px] font-bold text-[#00e5ff] tracking-wide">{{ roleLabel(report.role) }}</span>
            <span class="text-[11px] text-[#3a5a6a]">{{ durStr }}</span>
          </div>
        </div>

        <div class="h-px bg-[#102030] shrink-0"></div>

        <!-- Grade -->
        <div class="flex flex-col items-center gap-1.5 py-1 shrink-0">
          <span class="text-[9px] font-bold text-[#3a6a7a] tracking-[1.5px]">NOTA GERAL</span>
          <div class="w-16 h-16 border-2 rounded-lg flex items-center justify-center text-4xl font-black leading-none"
               :style="{ color: gc(report.overall_grade), borderColor: gc(report.overall_grade), boxShadow: `0 0 28px ${gc(report.overall_grade)}44` }">
            {{ report.overall_grade }}
          </div>
        </div>

        <div class="h-px bg-[#102030] shrink-0"></div>

        <!-- Metric list -->
        <div class="flex flex-col gap-2 flex-1 overflow-hidden">
          <div v-for="m in report.metrics" :key="m.label" class="flex items-center gap-1.5">
            <span class="text-[11px] font-black w-5.5 h-5.5 border rounded flex items-center justify-center shrink-0"
                  :style="{ color: gc(m.grade), borderColor: gc(m.grade) }">{{ m.grade }}</span>
            <div class="flex-1 flex flex-col gap-0.75 min-w-0">
              <div class="flex justify-between items-baseline">
                <span class="text-[10px] font-bold text-[#3a6a7a] tracking-wide truncate">{{ m.label.toUpperCase() }}</span>
                <span class="text-[11px] font-bold shrink-0" :style="{ color: gc(m.grade) }">
                  {{ fmt(m.player_value, m.unit) }}<span class="text-[10px] text-[#1a4a5a] font-normal">/ {{ fmt(m.benchmark_value, m.unit) }}</span>
                </span>
              </div>
              <div class="h-0.75 bg-[#0a1a2a] rounded overflow-hidden">
                <div class="h-full rounded transition-[width] duration-[900ms] ease-[cubic-bezier(0.23,1,0.32,1)]"
                     :style="{ width: fillPct(m), background: gc(m.grade) }"></div>
              </div>
            </div>
          </div>
        </div>
      </div>

      <!-- Right main -->
      <div class="flex-1 flex flex-col p-3 gap-2.5 overflow-hidden">

        <!-- Stat cards -->
        <div class="grid grid-cols-4 gap-2 shrink-0">
          <div v-for="m in report.metrics" :key="m.label"
               class="bg-[#060f1b] border border-[#102030] rounded-md p-2.5 flex flex-col gap-1">
            <div class="flex justify-between items-center mb-1">
              <span class="text-lg leading-none">{{ metricIcon(m.label) }}</span>
              <span class="text-[11px] font-black w-6 h-6 border-[1.5px] rounded flex items-center justify-center"
                    :style="{ color: gc(m.grade), borderColor: gc(m.grade), boxShadow: `0 0 8px ${gc(m.grade)}55` }">{{ m.grade }}</span>
            </div>
            <span class="text-[9px] font-bold text-[#3a6a7a] tracking-wider">{{ m.label.toUpperCase() }}</span>
            <span class="text-[26px] font-black leading-none" :style="{ color: gc(m.grade) }">{{ fmt(m.player_value, m.unit) }}</span>
            <span class="text-[10px] font-semibold" :style="{ color: gc(m.grade) }">{{ cardLabel(m) }}</span>
          </div>
        </div>

        <!-- AI callout -->
        <div class="shrink-0 p-3 px-3.5 bg-[rgba(0,229,255,0.03)] border border-[rgba(0,229,255,0.15)] rounded-md">
          <div class="flex items-center gap-2 mb-1">
            <span class="w-1.5 h-1.5 rounded-full bg-[#00e5ff] shadow-[0_0_10px_rgba(0,229,255,0.8)] shrink-0 animate-[blink_2s_ease-in-out_infinite]"></span>
            <span class="text-[10px] font-extrabold text-[#00e5ff] tracking-[2px]">RIFT SYNC AI — ANÁLISE DA PARTIDA</span>
          </div>
          <p class="text-[13px] text-[#7a9ab4] leading-relaxed italic m-0">"{{ report.priority_tip }}"</p>
        </div>

        <!-- Feedback -->
        <div class="flex-1 flex flex-col gap-2 overflow-y-auto [&::-webkit-scrollbar]:w-0.75 [&::-webkit-scrollbar-thumb]:bg-[#102030]">
          <div v-if="weakMetrics.length" class="flex flex-col gap-1.5">
            <div class="text-[10px] font-extrabold tracking-[1.5px] pb-1 border-b border-[#102030] text-[#ef5350]">▼ O QUE MELHORAR</div>
            <div v-for="m in weakMetrics" :key="m.label" class="flex items-start gap-2.5">
              <div class="flex flex-col items-center gap-1 shrink-0 pt-px">
                <span class="text-xl leading-none">{{ metricIcon(m.label) }}</span>
                <span class="text-[12px] font-black w-6.5 h-6.5 border-[1.5px] rounded flex items-center justify-center"
                      :style="{ color: gc(m.grade), borderColor: gc(m.grade), boxShadow: `0 0 8px ${gc(m.grade)}44` }">{{ m.grade }}</span>
              </div>
              <div class="flex flex-col gap-0.5 min-w-0">
                <span class="text-[12px] font-bold text-[#c8d8e8]">{{ m.label }}</span>
                <span class="text-[11px] text-[#00e5ff] opacity-65 tabular-nums">{{ metricContext(m) }}</span>
                <span v-if="m.history_note" class="text-[10px] text-[#4fc3f7] opacity-80 italic">📈 {{ m.history_note }}</span>
                <span class="text-[11px] text-[#5a7a8a] leading-snug">{{ m.feedback }}</span>
              </div>
            </div>
          </div>

          <div v-if="strongMetrics.length" class="flex flex-col gap-1.5">
            <div class="text-[10px] font-extrabold tracking-[1.5px] pb-1 border-b border-[#102030] text-[#00ff9d]">▲ PONTOS FORTES</div>
            <div v-for="m in strongMetrics" :key="m.label" class="flex items-start gap-2.5">
              <div class="flex flex-col items-center gap-1 shrink-0 pt-px">
                <span class="text-xl leading-none">{{ metricIcon(m.label) }}</span>
                <span class="text-[12px] font-black w-6.5 h-6.5 border-[1.5px] rounded flex items-center justify-center"
                      :style="{ color: gc(m.grade), borderColor: gc(m.grade), boxShadow: `0 0 8px ${gc(m.grade)}44` }">{{ m.grade }}</span>
              </div>
              <div class="flex flex-col gap-0.5 min-w-0">
                <span class="text-[12px] font-bold text-[#c8d8e8]">{{ m.label }}</span>
                <span class="text-[11px] text-[#00e5ff] opacity-65 tabular-nums">{{ metricContext(m) }}</span>
                <span v-if="m.history_note" class="text-[10px] text-[#4fc3f7] opacity-80 italic">📈 {{ m.history_note }}</span>
                <span class="text-[11px] text-[#5a7a8a] leading-snug">{{ m.feedback }}</span>
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
@keyframes blink { 0%,100% { opacity:1; } 50% { opacity:0.3; } }
</style>
