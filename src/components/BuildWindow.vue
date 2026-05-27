<script setup lang="ts">
import { ref, onMounted, watch } from 'vue';
import { listen } from '@tauri-apps/api/event';
import { invoke } from '@tauri-apps/api/core';
import { getCurrentWindow } from '@tauri-apps/api/window';

const hideWindow = () => getCurrentWindow().hide();

const props = defineProps<{
  champion?: string;
  items?: any[];
  version?: string;
}>();

const championName = ref('');
const recommendedItems = ref<any[]>([]);
const recommendedRunes = ref<any | null>(null);
const isVisible = ref(false);
const currentVersion = ref('16.10.1');
const activeRole = ref<string | null>(null);
const activeElo = ref<string | null>(null);

const RUNE_TREES: Record<number, { name: string, color: string }> = {
  8000: { name: 'Precisão',     color: '#C89B3C' },
  8100: { name: 'Dominação',    color: '#DC4747' },
  8200: { name: 'Feitiçaria',   color: '#4980F7' },
  8300: { name: 'Inspiração',   color: '#47AFDC' },
  8400: { name: 'Determinação', color: '#2FA84A' },
};

const KEYSTONES: Record<number, string> = {
  8005: 'Pressione o Ataque', 8008: 'Ritmo Fatal', 8021: 'Agilidade nos Pés',
  8010: 'Conquistador', 8112: 'Eletrocutar', 8124: 'Predador',
  8128: 'Colheita Sombria', 9923: 'Chuva de Lâminas', 8214: 'Invocação: Aery',
  8229: 'Cometa Arcano', 8230: 'Ímpeto Gradual', 8437: 'Aperto Morto-Vivo',
  8439: 'Pós-choque', 8465: 'Guardião', 8351: 'Aprimoramento Glac.',
  8360: 'Livro de Feitiços', 8369: 'Primeiro Ataque',
};

const getRuneTree = (id: any) => RUNE_TREES[Number(id)] || { name: 'Runas', color: '#A0A0A0' };
const getKeystone = (runesList: any) => {
  if (!runesList || !Array.isArray(runesList) || runesList.length === 0) return 'Destaque';
  return KEYSTONES[Number(runesList[0])] || 'Recomendada';
};

const updateBuild = async (targetChamp: string, role?: string | null, elo?: string | null) => {
  if (!targetChamp) return;
  const newChamp = targetChamp.replace(' ', '');
  isVisible.value = true;
  const roleChanged = role !== undefined && role !== activeRole.value;
  const eloChanged = elo !== undefined && elo !== activeElo.value;
  if (newChamp !== championName.value || roleChanged || eloChanged || recommendedItems.value.length === 0) {
    championName.value = newChamp;
    if (role !== undefined) activeRole.value = role;
    if (elo !== undefined) activeElo.value = elo;
    try {
      const builds: any[] = await invoke('get_recommended_builds_command', {
        champId: championName.value, role: activeRole.value || null, elo: activeElo.value || null,
      });
      if (builds && builds.length > 0) {
        const itemIds = typeof builds[0].items_json === 'string' ? JSON.parse(builds[0].items_json) : builds[0].items_json;
        recommendedItems.value = itemIds.map((id: any) => ({ item_id: id }));
        if (builds[0].runes_json) {
          recommendedRunes.value = typeof builds[0].runes_json === 'string' ? JSON.parse(builds[0].runes_json) : builds[0].runes_json;
        } else { recommendedRunes.value = null; }
      }
    } catch (e) { console.error(e); }
  }
};

watch(() => props.champion, async (newVal) => {
  if (newVal) {
    championName.value = newVal; isVisible.value = true;
    try {
      const builds: any[] = await invoke('get_recommended_builds_command', {
        champId: newVal, role: activeRole.value || null, elo: activeElo.value || null,
      });
      if (builds && builds.length > 0 && builds[0].runes_json) {
        recommendedRunes.value = typeof builds[0].runes_json === 'string' ? JSON.parse(builds[0].runes_json) : builds[0].runes_json;
      } else { recommendedRunes.value = null; }
    } catch (e) { console.error(e); }
  }
}, { immediate: true });

watch(() => props.items, (newVal) => {
  if (newVal) recommendedItems.value = newVal.map(id => typeof id === 'object' ? id : { item_id: id });
}, { immediate: true });

watch(() => props.version, (newVal) => { if (newVal) currentVersion.value = newVal; }, { immediate: true });

onMounted(async () => {
  if (props.champion) return;
  const waitForDb = async () => {
    try {
      const ready = await invoke('is_db_ready');
      if (ready) {
        const versions: string[] = await invoke('get_ddragon_versions') as any;
        if (versions && versions.length > 0) currentVersion.value = versions[0];
      } else { setTimeout(waitForDb, 200); }
    } catch { setTimeout(waitForDb, 200); }
  };
  waitForDb();

  await listen('lcu-update', (event: any) => {
    const data: any = event.payload;
    let role = null;
    const elo = data.elo || null;
    if (data.state?.ChampSelect) role = data.state.ChampSelect.role;
    if (data.championName) {
      updateBuild(data.championName, role, elo);
    } else if (data.gameData) {
      const activeSumm: string = data.gameData.activePlayer?.summonerName || '';
      const activeBase = activeSumm.split('#')[0]?.toLowerCase() || '';
      let champ: string = data.gameData.activePlayer?.championName || '';
      if (!champ && activeSumm) {
        const match = (data.gameData.allPlayers as any[] | undefined)?.find((p: any) => {
          const pName: string = p.summonerName || '';
          return pName === activeSumm || (activeBase && pName.split('#')[0]?.toLowerCase() === activeBase);
        });
        champ = match?.championName || '';
      }
      if (champ) updateBuild(champ, role, elo);
    }
  });
});
</script>

<template>
  <div v-if="isVisible"
       class="flex items-center w-117.5 h-13.5 bg-[rgba(5,10,20,0.95)] border border-[rgba(200,155,60,0.6)] px-2.5 rounded gap-3 relative box-border">

    <!-- Champion -->
    <div class="flex items-center gap-2 pr-2.5 border-r border-white/10 min-w-20">
      <img :src="`https://ddragon.leagueoflegends.com/cdn/${currentVersion}/img/champion/${championName}.png`"
           class="w-8 h-8 border border-[#c8aa6e] rounded-sm" />
      <span class="text-[10px] font-extrabold text-[#c8aa6e] uppercase">{{ championName }}</span>
    </div>

    <!-- Items -->
    <div class="flex gap-1">
      <div v-for="(item, idx) in recommendedItems" :key="idx"
           class="w-8 h-8 border border-white/20 bg-black overflow-hidden">
        <img :src="`https://ddragon.leagueoflegends.com/cdn/${currentVersion}/img/item/${item.item_id}.png`"
             class="w-full h-full" />
      </div>
    </div>

    <!-- Runes -->
    <div v-if="recommendedRunes"
         class="flex flex-col justify-center pl-2.5 border-l border-white/10 h-8 gap-0.5 min-w-27.5">
      <div class="flex items-center gap-1 text-[9px] font-extrabold uppercase"
           :style="{ color: getRuneTree(recommendedRunes.primary_tree).color }">
        <span class="opacity-80 text-[8px] bg-white/5 px-0.5 rounded-sm">{{ getRuneTree(recommendedRunes.primary_tree).name }}</span>
        <span class="text-white font-bold text-[10px] normal-case whitespace-nowrap">{{ getKeystone(recommendedRunes.runes) }}</span>
      </div>
      <div class="text-[8px] text-white/50 font-semibold uppercase">+ {{ getRuneTree(recommendedRunes.secondary_tree).name }}</div>
    </div>

    <!-- Brand -->
    <div class="absolute right-7 bottom-0.5 text-[7px] font-extrabold text-[#c8aa6e] opacity-40 tracking-widest">SPELL COACH IA</div>

    <!-- Close -->
    <button class="absolute right-1.5 top-1/2 -translate-y-1/2 bg-none border-none text-white/25 text-base cursor-pointer px-0.5 rounded-sm leading-none hover:text-[#ff4e4e] transition-colors"
            @click="hideWindow">×</button>
  </div>
</template>
