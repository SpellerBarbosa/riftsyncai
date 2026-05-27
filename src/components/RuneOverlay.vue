<script setup lang="ts">
import { ref, onMounted, onUnmounted, computed } from 'vue';
import { listen, emit } from '@tauri-apps/api/event';
import {
  RUNE_COLORS, TREE_STRUCTURES, SHARDS_ROWS,
  getFullUrl, getTreeHeaderIcon, normalizeRuneId
} from './runesData';

const closeWindow = async () => {
  try { await emit('hide-rune-overlay'); }
  catch (e) { console.error("Erro ao emitir hide-rune-overlay:", e); }
};

const championName      = ref('Campeão');
const primaryTreeName   = ref('Precisão');
const primaryTreeId     = ref(8000);
const secondaryTreeName = ref('Dominação');
const secondaryTreeId   = ref(8100);
const keystoneName      = ref('Conquistador');
const keystoneId        = ref(8010);
const runesList         = ref<number[]>([]);
const shardsList        = ref<number[]>([]);
const activeColor       = ref('#C89B3C');

const primaryTreeObj   = computed(() => TREE_STRUCTURES[primaryTreeId.value]   || TREE_STRUCTURES[8000]);
const secondaryTreeObj = computed(() => TREE_STRUCTURES[secondaryTreeId.value] || TREE_STRUCTURES[8100]);

let unlistenUpdate: any;
onMounted(async () => {
  unlistenUpdate = await listen('update-rune-overlay-content', (event: any) => {
    const d = event.payload;
    championName.value      = d.champion_name;
    primaryTreeName.value   = d.primary_tree_name;
    primaryTreeId.value     = d.primary_tree_id;
    secondaryTreeName.value = d.secondary_tree_name;
    secondaryTreeId.value   = d.secondary_tree_id;
    keystoneName.value      = d.keystone_name;
    keystoneId.value        = d.keystone_id;
    runesList.value  = (d.runes  || []).map((id: number) => normalizeRuneId(id));
    shardsList.value = d.shards || [];
    activeColor.value = RUNE_COLORS[d.primary_tree_id] || '#C89B3C';
  });
  emit('request-rune-overlay-content');
  window.addEventListener('keydown', onKey);
});
const onKey = (e: KeyboardEvent) => { if (e.key === 'Escape') closeWindow(); };
onUnmounted(() => { if (unlistenUpdate) unlistenUpdate(); window.removeEventListener('keydown', onKey); });
</script>

<template>
  <!-- Card: fixed size, flex column -->
  <div class="w-115 h-95 flex flex-col rounded-lg p-3 px-4 relative overflow-hidden select-none transition-all duration-500"
       :style="{
         '--accent-color': activeColor,
         background: 'linear-gradient(135deg,rgba(3,8,16,0.98) 0%,rgba(1,4,8,0.99) 100%)',
         border: `1px solid ${activeColor}`,
         boxShadow: '0 15px 35px rgba(0,0,0,0.9),inset 0 0 20px rgba(0,0,0,0.5)',
       }">

    <!-- Header -->
    <div class="flex justify-between items-center mb-3 border-b border-white/8 pb-2 shrink-0">
      <div class="flex flex-col gap-0.5">
        <span class="text-[8px] font-extrabold text-[#a09b8c] tracking-[2px] uppercase">PÁGINA DE RUNAS IA</span>
        <h2 class="text-lg font-black m-0 tracking-wide uppercase [text-shadow:0_0_12px_rgba(255,255,255,0.1)]"
            :style="{ color: activeColor }">{{ championName }}</h2>
      </div>
      <div class="flex items-center gap-2.5">
        <button class="bg-none border-none text-[#cdbe91] text-xl font-light cursor-pointer opacity-60 w-5 h-5 flex items-center justify-center rounded transition-all hover:opacity-100 hover:text-[#ff4e4e] hover:bg-[rgba(255,78,78,0.1)]"
                @click="closeWindow" title="Fechar">×</button>
        <div class="w-2 h-2 rounded-full" :style="{ background: activeColor, boxShadow: `0 0 10px ${activeColor}` }"></div>
      </div>
    </div>

    <!-- Body: two columns -->
    <div class="flex-1 grid grid-cols-2 gap-5 min-h-0 items-start">

      <!-- Primary tree -->
      <div class="flex flex-col gap-2.5">
        <div class="flex items-center gap-2 border-b border-white/4 pb-1">
          <img :src="getTreeHeaderIcon(primaryTreeId)" class="w-5 h-5 object-contain" />
          <span class="text-[11px] font-extrabold uppercase tracking-wider" :style="{ color: activeColor }">{{ primaryTreeName }}</span>
        </div>
        <div class="flex flex-col gap-3 items-center py-1.5 px-1 bg-white/1 rounded border border-white/2">
          <!-- Keystones -->
          <div class="flex justify-center gap-2.5 w-full">
            <div v-for="k in primaryTreeObj.keystones" :key="k.id"
                 class="w-9 h-9 rounded-full flex items-center justify-center bg-black/65 border transition-all duration-250 cursor-pointer relative"
                 :class="runesList.includes(k.id)
                   ? 'border-current scale-110 z-2 shadow-[0_0_15px_currentColor]'
                   : 'border-white/10 opacity-15 grayscale contrast-[0.85]'"
                 :style="runesList.includes(k.id) ? { borderColor: activeColor, boxShadow: `0 0 15px ${activeColor}` } : {}"
                 :title="k.name">
              <img :src="getFullUrl(k.icon)" class="w-4/5 h-4/5 object-contain" />
            </div>
          </div>
          <!-- Sub-rune rows -->
          <div v-for="(row, ri) in primaryTreeObj.rows" :key="ri" class="flex justify-center gap-3.5 w-full">
            <div v-for="r in row" :key="r.id"
                 class="w-7 h-7 rounded-full flex items-center justify-center bg-black/65 border transition-all duration-250 cursor-pointer"
                 :class="runesList.includes(r.id)
                   ? 'scale-110 z-2 border-current'
                   : 'border-white/10 opacity-15 grayscale'"
                 :style="runesList.includes(r.id) ? { borderColor: activeColor, boxShadow: `0 0 10px ${activeColor}` } : {}"
                 :title="r.name">
              <img :src="getFullUrl(r.icon)" class="w-4/5 h-4/5 object-contain" />
            </div>
          </div>
        </div>
      </div>

      <!-- Secondary tree + shards -->
      <div class="flex flex-col gap-2.5">
        <div class="flex items-center gap-2 border-b border-white/4 pb-1">
          <img :src="getTreeHeaderIcon(secondaryTreeId)" class="w-4 h-4 object-contain" />
          <span class="text-[11px] font-extrabold uppercase tracking-wider text-[#a09b8c]">{{ secondaryTreeName }}</span>
        </div>
        <div class="flex flex-col gap-3 items-center py-1.5 px-1 bg-white/1 rounded border border-white/2">
          <div v-for="(row, ri) in secondaryTreeObj.rows" :key="ri" class="flex justify-center gap-3 w-full">
            <div v-for="r in row" :key="r.id"
                 class="w-6 h-6 rounded-full flex items-center justify-center bg-black/65 border transition-all duration-200 cursor-pointer"
                 :class="runesList.includes(r.id)
                   ? 'scale-115 z-2'
                   : 'opacity-20 grayscale'"
                 :style="runesList.includes(r.id)
                   ? { borderColor: RUNE_COLORS[secondaryTreeId] || '#cdbe91', boxShadow: `0 0 6px ${RUNE_COLORS[secondaryTreeId] || '#cdbe91'}` }
                   : { borderColor: 'rgba(255,255,255,0.1)' }"
                 :title="r.name">
              <img :src="getFullUrl(r.icon)" class="w-4/5 h-4/5 object-contain" />
            </div>
          </div>
        </div>

        <!-- Shards -->
        <div class="mt-3.5 p-2 px-3 bg-white/2 rounded border border-white/3 flex flex-col gap-2">
          <span class="text-[8px] font-extrabold uppercase text-[#a09b8c] tracking-[1.5px] border-b border-white/3 pb-1">Atributos</span>
          <div class="flex flex-col gap-2 items-center">
            <div v-for="(row, ri) in SHARDS_ROWS" :key="ri" class="flex justify-center gap-3 w-full">
              <div v-for="s in row" :key="s.id"
                   class="w-4 h-4 rounded-full bg-black/70 border flex items-center justify-center transition-all duration-200"
                   :class="shardsList[ri] === s.id
                     ? 'border-[#cdbe91] shadow-[0_0_6px_#cdbe91] scale-115 bg-black/90'
                     : 'border-white/10 opacity-20 grayscale'"
                   :title="s.name">
                <img :src="getFullUrl(s.icon)" class="w-[70%] h-[70%] object-contain" />
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>

    <!-- Footer -->
    <div class="mt-3 relative flex items-center justify-between pt-2 shrink-0">
      <div class="absolute top-0 left-0 w-full h-px opacity-30" :style="{ background: activeColor }"></div>
      <span class="text-[7px] font-extrabold text-[#cdbe91] opacity-50 tracking-wider">SPELL COACH IA • PÁGINA MONTADA</span>
    </div>
  </div>
</template>
