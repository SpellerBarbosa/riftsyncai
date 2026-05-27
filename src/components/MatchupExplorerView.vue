<script setup lang="ts">
import BuildWindow from "./BuildWindow.vue";
import Flashcard from "./Flashcard.vue";
import {
  TREE_STRUCTURES,
  RUNE_COLORS,
  getFullUrl,
  getTreeHeaderIcon,
} from './runesData';

defineProps<{
  resolvedChampId: string;
  bestMatchups: any[];
  worstMatchups: any[];
  coreBuild: number[];
  situationalItems: any[];
  championRunes: any;
  tacticalTips: { matchup_front: string; matchup_back: string; item_front: string; item_back: string };
  ddVersion: string;
  findRuneDetails: (id: number) => any;
  findShardDetails: (id: number) => any;
}>();
</script>

<template>
  <div class="flex flex-col gap-6 animate-[fadeIn_0.3s_ease-out] max-w-[900px] mx-auto">

    <!-- Header -->
    <div class="flex justify-between items-center p-4 bg-linear-to-r from-[rgba(200,155,60,0.1)] to-transparent border-l-4 border-l-[#c89b3c]">
      <div class="flex items-center gap-4">
        <img :src="`https://ddragon.leagueoflegends.com/cdn/${ddVersion || '16.10.1'}/img/champion/${resolvedChampId}.png`"
             class="w-16 h-16 border-2 border-[#c89b3c] rounded" />
        <div class="flex flex-col">
          <h2 class="text-[#c89b3c] uppercase tracking-[2px] m-0 font-bold text-xl">{{ resolvedChampId }}</h2>
          <span class="text-[10px] text-[#a09b8c] uppercase tracking-[1px]">Análise Tática de Matchups e Builds</span>
        </div>
      </div>
      <span class="text-[11px] text-[#a09b8c]">{{ coreBuild.length }} Itens Core</span>
    </div>

    <!-- Build Section -->
    <section class="bg-white/2 p-4 rounded border border-white/5">
      <div class="flex items-center gap-2.5 mb-3">
        <span class="text-lg leading-none">⚔️</span>
        <h3 class="text-[12px] uppercase text-[#c89b3c] m-0 font-bold">Build Recomendada (Core)</h3>
      </div>
      <div class="flex gap-2">
        <div v-for="id in coreBuild" :key="id" :title="`Item ID: ${id}`">
          <img :src="`https://ddragon.leagueoflegends.com/cdn/${ddVersion || '16.10.1'}/img/item/${id}.png`"
               class="w-9 h-9 border border-[rgba(200,155,60,0.3)] rounded bg-black/40"
               @error="(e:any) => e.target.style.display='none'" />
        </div>
      </div>
    </section>

    <!-- Runes Section -->
    <section v-if="championRunes" class="bg-[rgba(3,8,16,0.45)] border border-[rgba(200,155,60,0.15)] rounded-lg p-4 shadow-[inset_0_0_12px_rgba(0,0,0,0.6)]">
      <div class="flex items-center gap-2.5 mb-3">
        <span class="text-lg leading-none">🔮</span>
        <h3 class="text-[12px] uppercase text-[#c89b3c] m-0 font-bold">Runas Recomendadas</h3>
      </div>
      <div class="grid grid-cols-2 gap-4 mt-3">
        <!-- Primary Tree -->
        <div class="bg-[rgba(1,4,8,0.5)] border border-white/3 rounded-md p-3 flex flex-col gap-2.5"
             :style="{ borderLeft: `3px solid ${RUNE_COLORS[championRunes.primary_tree] || '#C89B3C'}` }">
          <div class="flex items-center gap-2 border-b border-white/5 pb-1.5">
            <img :src="getTreeHeaderIcon(championRunes.primary_tree)" class="w-5 h-5 object-contain" />
            <span class="text-[12px] font-extrabold uppercase tracking-[0.5px]"
                  :style="{ color: RUNE_COLORS[championRunes.primary_tree] || '#C89B3C' }">
              {{ TREE_STRUCTURES[championRunes.primary_tree]?.name || 'Primária' }}
            </span>
          </div>
          <div class="flex flex-col gap-1.5">
            <div v-if="championRunes.runes[0] > 0"
                 class="flex items-center gap-2.5 px-2.5 py-1.5 bg-[rgba(200,155,60,0.05)] border border-[rgba(200,155,60,0.2)] rounded">
              <img :src="getFullUrl(findRuneDetails(championRunes.runes[0])?.icon || '')" class="w-8 h-8 object-contain" />
              <div class="flex flex-col gap-px">
                <span class="text-[8px] text-[#c89b3c] uppercase font-bold tracking-[0.5px]">Runa Principal</span>
                <span class="text-[11px] text-[#f0e6d2] font-semibold">{{ findRuneDetails(championRunes.runes[0])?.name }}</span>
              </div>
            </div>
            <template v-for="runeId in championRunes.runes.slice(1, 4)" :key="runeId">
              <div v-if="runeId > 0" class="flex items-center gap-2.5 px-2.5 py-1.5 bg-white/2 border border-white/4 rounded">
                <img :src="getFullUrl(findRuneDetails(runeId)?.icon || '')" class="w-5 h-5 object-contain" />
                <span class="text-[11px] text-[#f0e6d2] font-semibold">{{ findRuneDetails(runeId)?.name }}</span>
              </div>
            </template>
          </div>
        </div>

        <!-- Secondary Tree & Shards -->
        <div class="bg-[rgba(1,4,8,0.5)] border border-white/3 rounded-md p-3 flex flex-col gap-2.5"
             :style="{ borderLeft: `3px solid ${RUNE_COLORS[championRunes.secondary_tree] || '#a09b8c'}` }">
          <div class="flex items-center gap-2 border-b border-white/5 pb-1.5">
            <img :src="getTreeHeaderIcon(championRunes.secondary_tree)" class="w-4 h-4 object-contain" />
            <span class="text-[12px] font-extrabold uppercase tracking-[0.5px]"
                  :style="{ color: RUNE_COLORS[championRunes.secondary_tree] || '#a09b8c' }">
              {{ TREE_STRUCTURES[championRunes.secondary_tree]?.name || 'Secundária' }}
            </span>
          </div>
          <div class="flex flex-col gap-1.5">
            <template v-for="runeId in championRunes.runes.slice(4, 6)" :key="runeId">
              <div v-if="runeId > 0" class="flex items-center gap-2.5 px-2.5 py-1.5 bg-white/2 border border-white/4 rounded">
                <img :src="getFullUrl(findRuneDetails(runeId)?.icon || '')" class="w-5 h-5 object-contain" />
                <span class="text-[11px] text-[#f0e6d2] font-semibold">{{ findRuneDetails(runeId)?.name }}</span>
              </div>
            </template>
          </div>
          <div v-if="championRunes.shards && championRunes.shards.length > 0"
               class="border-t border-white/5 pt-2 flex flex-col gap-1.5">
            <span class="text-[9px] font-bold text-[#a09b8c] uppercase tracking-[0.5px]">Atributos</span>
            <div class="flex gap-2">
              <div v-for="(shardId, idx) in championRunes.shards" :key="idx"
                   v-show="findShardDetails(shardId)"
                   class="w-5 h-5 rounded-full bg-black/50 border border-white/10 flex items-center justify-center"
                   :title="findShardDetails(shardId)?.name">
                <img :src="getFullUrl(findShardDetails(shardId)?.icon || '')" class="w-[70%] h-[70%] object-contain" />
              </div>
            </div>
          </div>
        </div>
      </div>
    </section>

    <!-- Matchup Grids -->
    <div class="grid grid-cols-2 gap-5">
      <section>
        <h3 class="text-[12px] text-[#c89b3c] uppercase mb-2 font-bold">↑ Melhores Confrontos</h3>
        <div class="flex flex-col gap-2">
          <div v-for="m in bestMatchups" :key="m.opponent_id"
               class="flex items-center gap-3 bg-black/25 p-2 rounded border border-white/3">
            <img :src="`https://ddragon.leagueoflegends.com/cdn/${ddVersion || '16.10.1'}/img/champion/${m.opponent_id}.png`"
                 class="w-10 h-10 rounded" />
            <div class="flex-1">
              <span class="block text-[11px] font-bold mb-1">{{ m.opponent_id }}</span>
              <div class="h-1 bg-white/5 rounded overflow-hidden mb-1">
                <div class="h-full rounded" :style="{ width: (m.win_rate * 100) + '%', background: '#4eff9b' }"></div>
              </div>
              <div class="flex justify-between items-center text-[10px] font-extrabold mt-0.5">
                <span class="text-[#4eff9b]">{{ (m.win_rate * 100).toFixed(1) }}% WR</span>
                <span v-if="m.games_count" class="text-[#a09b8c] font-mono text-[9px] opacity-80">V: {{ m.wins_count || 0 }} | D: {{ (m.games_count || 0) - (m.wins_count || 0) }}</span>
              </div>
            </div>
          </div>
        </div>
      </section>

      <section>
        <h3 class="text-[12px] text-[#c89b3c] uppercase mb-2 font-bold">↓ Piores Confrontos (Counters)</h3>
        <div class="flex flex-col gap-2">
          <div v-for="m in worstMatchups" :key="m.opponent_id"
               class="flex items-center gap-3 bg-black/25 p-2 rounded border border-white/3">
            <img :src="`https://ddragon.leagueoflegends.com/cdn/${ddVersion || '16.10.1'}/img/champion/${m.opponent_id}.png`"
                 class="w-10 h-10 rounded" />
            <div class="flex-1">
              <span class="block text-[11px] font-bold mb-1">{{ m.opponent_id }}</span>
              <div class="h-1 bg-white/5 rounded overflow-hidden mb-1">
                <div class="h-full rounded" :style="{ width: ((1.0 - m.win_rate) * 100) + '%', background: '#ff4e4e' }"></div>
              </div>
              <div class="flex justify-between items-center text-[10px] font-extrabold mt-0.5">
                <span class="text-[#ff4e4e]">{{ ((1.0 - m.win_rate) * 100).toFixed(1) }}% WR (Op.)</span>
                <span v-if="m.games_count" class="text-[#a09b8c] font-mono text-[9px] opacity-80">V: {{ m.wins_count || 0 }} | D: {{ (m.games_count || 0) - (m.wins_count || 0) }}</span>
              </div>
            </div>
          </div>
        </div>
      </section>
    </div>

    <!-- Situational Items -->
    <section v-if="situationalItems.length > 0" class="bg-white/2 p-4 rounded border border-white/5">
      <div class="flex items-center gap-2.5 mb-3">
        <span class="text-lg leading-none">💎</span>
        <h3 class="text-[12px] uppercase text-[#c89b3c] m-0 font-bold">Itens Situacionais e Transição</h3>
      </div>
      <div class="grid grid-cols-4 gap-3">
        <div v-for="item in situationalItems.slice(0, 8)" :key="item.item_id"
             class="flex items-center gap-2 bg-black/20 p-1.5 rounded">
          <img :src="`https://ddragon.leagueoflegends.com/cdn/${ddVersion || '16.10.1'}/img/item/${item.item_id}.png`"
               class="w-8 h-8 rounded-sm" />
          <div class="flex flex-col">
            <span class="text-[8px] text-[#a09b8c]">Slot: {{ item.slot_type }}</span>
            <span class="text-[10px] font-extrabold text-[#4eff9b]">{{ (item.win_rate * 100).toFixed(1) }}% WR</span>
          </div>
        </div>
      </div>
    </section>

    <!-- Overlay Preview -->
    <section class="border-t border-white/10 pt-6">
      <div class="flex items-center gap-2.5 mb-3">
        <span class="text-lg leading-none">👁️</span>
        <h3 class="text-[12px] uppercase text-[#c89b3c] m-0 font-bold">Simulação do Overlay (In-Game)</h3>
      </div>
      <div class="preview-container flex justify-center p-10 rounded-lg relative overflow-hidden">
        <div class="relative z-[1] w-[350px]">
          <BuildWindow :champion="resolvedChampId" :items="coreBuild" :version="ddVersion" />
        </div>
      </div>
    </section>

    <!-- Flashcards -->
    <section class="border-t border-white/10 pt-6 mb-10">
      <div class="flex items-center gap-2.5 mb-3">
        <span class="text-lg leading-none">🃏</span>
        <h3 class="text-[12px] uppercase text-[#c89b3c] m-0 font-bold">Dicas Táticas (Flashcards)</h3>
      </div>
      <div class="flex flex-wrap gap-5 justify-center py-5">
        <Flashcard title="Dica de Matchup" :frontText="tacticalTips.matchup_front" :backText="tacticalTips.matchup_back" rarity="epic" />
        <Flashcard title="Dica de Item" :frontText="tacticalTips.item_front" :backText="tacticalTips.item_back" rarity="legendary" />
      </div>
    </section>
  </div>
</template>

<style scoped>
@keyframes fadeIn {
  from { opacity: 0; transform: translateY(10px); }
  to   { opacity: 1; transform: translateY(0); }
}
.preview-container::before {
  content: '';
  position: absolute;
  inset: 0;
  background: rgba(0, 0, 0, 0.6);
  backdrop-filter: blur(4px);
}
</style>
