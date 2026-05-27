<script setup lang="ts">
defineProps<{
  styleProfile: any;
  analyzedCoaching: any;
  isAnalyzingProfile: boolean;
  ddVersion: string;
}>();
</script>

<template>
  <!-- Loading -->
  <div v-if="isAnalyzingProfile" class="flex flex-col items-center justify-center h-4/5 text-[#c89b3c] font-extrabold text-[13px] text-center">
    <div class="w-12 h-12 border-[3px] border-[rgba(200,155,60,0.1)] border-t-[#c89b3c] rounded-full animate-spin shadow-[0_0_10px_rgba(200,155,60,0.2)] mb-4"></div>
    <p class="mt-3">Persistindo dados no SQLite e gerando auditoria tática com IA...</p>
  </div>

  <!-- Profile data -->
  <div v-else-if="styleProfile" class="flex flex-col gap-5 p-2.5 overflow-y-auto" style="height: calc(100vh - 40px)">

    <!-- Profile Card -->
    <div class="hextech-profile-card bg-linear-to-br from-[rgba(30,35,40,0.95)] to-[rgba(1,10,19,0.98)] border border-[#c89b3c] p-4 rounded-md relative shadow-[0_4px_20px_rgba(0,0,0,0.5)] flex flex-col items-center text-center">
      <div class="flex items-center gap-2 bg-[rgba(200,155,60,0.1)] border border-[#c89b3c] px-4 py-1 rounded-[20px] mb-3 shadow-[0_0_10px_rgba(200,155,60,0.15)]">
        <span class="text-[#c89b3c] text-[12px] animate-pulse">⚡</span>
        <span class="text-sm font-black tracking-[1px] text-[#f0e6d2] uppercase">{{ styleProfile.style_tag }}</span>
      </div>
      <p class="text-[11px] leading-relaxed text-[#a09b8c] max-w-[500px] m-0">{{ styleProfile.style_description }}</p>
    </div>

    <!-- Performance Metrics -->
    <section class="bg-[rgba(30,35,40,0.4)] border border-white/5 rounded p-4">
      <div class="flex items-center gap-2.5 mb-3">
        <span class="text-lg leading-none">📈</span>
        <h3 class="text-[12px] uppercase text-[#c89b3c] m-0 font-bold">Métricas de Desempenho e Benchmarks</h3>
      </div>
      <div class="grid grid-cols-2 gap-4 mt-3">
        <div class="flex flex-col gap-1.5">
          <div class="flex justify-between text-[10px] font-extrabold uppercase text-[#a09b8c]">
            <span>Farm por Minuto</span>
            <strong class="text-[#f0e6d2] text-[11px] normal-case">{{ styleProfile.avg_cs_per_min.toFixed(1) }} CS/m</strong>
          </div>
          <div class="h-1.5 bg-black/30 rounded overflow-hidden border border-white/3">
            <div class="h-full rounded"
                 style="transition: width 1s cubic-bezier(0.1,0.8,0.1,1); background: linear-gradient(90deg, #c89b3c, #f0e6d2)"
                 :style="{ width: Math.min(100, (styleProfile.avg_cs_per_min / 9.0) * 100) + '%' }"></div>
          </div>
          <span class="text-[9px] text-[rgba(160,155,140,0.6)] font-bold">Meta Challenger: 8.5+ CS/min</span>
        </div>

        <div class="flex flex-col gap-1.5">
          <div class="flex justify-between text-[10px] font-extrabold uppercase text-[#a09b8c]">
            <span>Visão por Minuto</span>
            <strong class="text-[#f0e6d2] text-[11px] normal-case">{{ styleProfile.avg_vision_score_per_min.toFixed(2) }} Score/m</strong>
          </div>
          <div class="h-1.5 bg-black/30 rounded overflow-hidden border border-white/3">
            <div class="h-full rounded"
                 style="transition: width 1s cubic-bezier(0.1,0.8,0.1,1); background: linear-gradient(90deg, #008080, #00bff3)"
                 :style="{ width: Math.min(100, (styleProfile.avg_vision_score_per_min / 1.5) * 100) + '%' }"></div>
          </div>
          <span class="text-[9px] text-[rgba(160,155,140,0.6)] font-bold">Meta Challenger: 1.2+ Score/min</span>
        </div>

        <div class="flex flex-col gap-1.5">
          <div class="flex justify-between text-[10px] font-extrabold uppercase text-[#a09b8c]">
            <span>KDA Pessoal</span>
            <strong class="text-[#f0e6d2] text-[11px] normal-case">{{ styleProfile.avg_kda.toFixed(2) }} : 1</strong>
          </div>
          <div class="h-1.5 bg-black/30 rounded overflow-hidden border border-white/3">
            <div class="h-full rounded"
                 style="transition: width 1s cubic-bezier(0.1,0.8,0.1,1); background: linear-gradient(90deg, #4c85ff, #8ab4ff)"
                 :style="{ width: Math.min(100, (styleProfile.avg_kda / 4.0) * 100) + '%' }"></div>
          </div>
          <span class="text-[9px] text-[rgba(160,155,140,0.6)] font-bold">Meta Challenger: 3.5+ KDA</span>
        </div>

        <div class="flex flex-col gap-1.5">
          <div class="flex justify-between text-[10px] font-extrabold uppercase text-[#a09b8c]">
            <span>Média de Mortes</span>
            <strong class="text-[#f0e6d2] text-[11px] normal-case">{{ styleProfile.avg_deaths.toFixed(1) }} Mortes</strong>
          </div>
          <div class="h-1.5 bg-black/30 rounded overflow-hidden border border-white/3">
            <div class="h-full rounded"
                 style="transition: width 1s cubic-bezier(0.1,0.8,0.1,1); background: linear-gradient(90deg, #b30000, #ff4e4e)"
                 :style="{ width: Math.min(100, (styleProfile.avg_deaths / 10.0) * 100) + '%' }"></div>
          </div>
          <span class="text-[9px] text-[rgba(255,78,78,0.6)] font-bold">Meta Challenger: &lt; 4.5 mortes</span>
        </div>
      </div>
    </section>

    <!-- AI Coaching -->
    <div v-if="analyzedCoaching" class="grid grid-cols-2 gap-4">
      <section class="bg-[rgba(30,35,40,0.35)] border border-white/5 border-l-[3px] border-l-[#ff4e4e] rounded p-4">
        <div class="flex items-center gap-2.5 mb-3">
          <span class="text-lg leading-none">⚠️</span>
          <h3 class="text-[12px] uppercase text-[#ff4e4e] m-0 font-bold">Erros Recorrentes Detectados</h3>
        </div>
        <ul class="list-none p-0 mt-3 m-0 flex flex-col gap-2.5">
          <li v-for="(err, idx) in analyzedCoaching.erros" :key="idx"
              class="text-[11px] leading-snug text-[#f0e6d2] flex items-start gap-2">
            <span class="text-[#ff4e4e] font-black shrink-0">✖</span> {{ err }}
          </li>
        </ul>
      </section>

      <section class="bg-[rgba(30,35,40,0.35)] border border-white/5 border-l-[3px] border-l-[#c89b3c] rounded p-4">
        <div class="flex items-center gap-2.5 mb-3">
          <span class="text-lg leading-none">🎯</span>
          <h3 class="text-[12px] uppercase text-[#c89b3c] m-0 font-bold">Treinos Práticos do Challenger Coach</h3>
        </div>
        <ul class="list-none p-0 mt-3 m-0 flex flex-col gap-2.5">
          <li v-for="(drill, idx) in analyzedCoaching.treinos" :key="idx"
              class="text-[11px] leading-snug text-[#f0e6d2] flex items-center gap-2.5">
            <div class="w-3.5 h-3.5 rounded-full border border-[#c89b3c] text-[#c89b3c] flex items-center justify-center text-[8px] font-black bg-[rgba(200,155,60,0.1)] shrink-0">✓</div>
            <div class="flex-1">{{ drill }}</div>
          </li>
        </ul>
      </section>
    </div>

    <!-- Historic Matches -->
    <section class="bg-[rgba(30,35,40,0.3)] border border-white/4 rounded p-4">
      <div class="flex items-center gap-2.5 mb-3">
        <span class="text-lg leading-none">💾</span>
        <h3 class="text-[12px] uppercase text-[#c89b3c] m-0 font-bold">Histórico Persistido no SQLite</h3>
      </div>
      <div class="flex flex-col gap-2 mt-3">
        <div v-for="m in styleProfile.recent_matches" :key="m.match_id"
             class="grid items-center px-3 py-2 bg-black/20 rounded border border-white/2"
             :class="m.win ? 'border-l-[3px] border-l-[#4eff9b]' : 'border-l-[3px] border-l-[#ff4e4e]'"
             style="grid-template-columns: 150px 100px 150px 1fr">
          <div class="flex items-center gap-2">
            <img :src="`https://ddragon.leagueoflegends.com/cdn/${ddVersion || '16.10.1'}/img/champion/${m.champion_name}.png`"
                 class="w-7 h-7 rounded-sm border border-white/10"
                 @error="(e:any) => e.target.src='https://raw.communitydragon.org/latest/plugins/rcp-be-lol-game-data/global/default/v1/champion-icons/266.png'" />
            <div class="flex flex-col">
              <span class="text-[11px] font-bold text-[#f0e6d2]">{{ m.champion_name }}</span>
              <span class="text-[8px] uppercase text-[#a09b8c]">{{ m.position }}</span>
            </div>
          </div>
          <div class="flex flex-col">
            <span class="text-[11px] font-extrabold text-[#f0e6d2]">{{ m.kills }} / {{ m.deaths }} / {{ m.assists }}</span>
            <span class="text-[9px] text-[#a09b8c]">{{ ((m.kills + m.assists) / Math.max(1, m.deaths)).toFixed(2) }} KDA</span>
          </div>
          <div class="flex flex-col text-[10px] text-[#a09b8c]">
            <span>🌾 {{ m.cs_per_min.toFixed(1) }} CS/m</span>
            <span>👁️ {{ m.vision_score_per_min.toFixed(2) }} Vis/m</span>
          </div>
          <div class="text-right text-[10px] font-black tracking-[0.5px]"
               :class="m.win ? 'text-[#4eff9b]' : 'text-[#ff4e4e]'">
            {{ m.win ? 'VITÓRIA' : 'DERROTA' }}
          </div>
        </div>
      </div>
    </section>
  </div>

  <!-- Empty state -->
  <div v-else class="flex flex-col items-center justify-center text-center h-4/5 text-[#a09b8c] animate-[fadeIn_0.4s_ease-out_forwards]">
    <div class="text-5xl mb-4 text-[#c89b3c] animate-pulse">⚜️</div>
    <h2 class="text-[16px] text-[#f0e6d2] mb-2">Conecte e Carregue seu Estilo</h2>
    <p class="text-[11px] max-w-[320px] leading-relaxed m-0">
      Para gerar sua auditoria tática personalizada e descobrir erros recorrentes, clique no botão
      <strong class="text-[#c89b3c]">"Analisar Estilo"</strong> no menu lateral.
    </p>
  </div>
</template>

<style scoped>
@keyframes fadeIn {
  from { opacity: 0; transform: translateY(10px); }
  to   { opacity: 1; transform: translateY(0); }
}
.hextech-profile-card::before {
  content: "⚜️";
  position: absolute;
  top: -12px;
  font-size: 16px;
  color: #c89b3c;
  background: #010a13;
  padding: 0 8px;
}
</style>
