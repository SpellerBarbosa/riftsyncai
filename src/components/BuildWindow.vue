<script setup lang="ts">
import { ref, onMounted, watch } from 'vue';
import { listen } from '@tauri-apps/api/event';
import { invoke } from '@tauri-apps/api/core';

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
  8000: { name: 'Precisão', color: '#C89B3C' },
  8100: { name: 'Dominação', color: '#DC4747' },
  8200: { name: 'Feitiçaria', color: '#4980F7' },
  8300: { name: 'Inspiração', color: '#47AFDC' },
  8400: { name: 'Determinação', color: '#2FA84A' }
};

const KEYSTONES: Record<number, string> = {
  8005: 'Pressione o Ataque',
  8008: 'Ritmo Fatal',
  8021: 'Agilidade nos Pés',
  8010: 'Conquistador',
  8112: 'Eletrocutar',
  8124: 'Predador',
  8128: 'Colheita Sombria',
  9923: 'Chuva de Lâminas',
  8214: 'Invocação: Aery',
  8229: 'Cometa Arcano',
  8230: 'Ímpeto Gradual',
  8437: 'Aperto Morto-Vivo',
  8439: 'Pós-choque',
  8465: 'Guardião',
  8351: 'Aprimoramento Glac.',
  8360: 'Livro de Feitiços',
  8369: 'Primeiro Ataque'
};

const getRuneTree = (id: any) => {
  const numId = Number(id);
  return RUNE_TREES[numId] || { name: 'Runas', color: '#A0A0A0' };
};

const getKeystone = (runesList: any) => {
  if (!runesList || !Array.isArray(runesList) || runesList.length === 0) return 'Destaque';
  const firstRune = Number(runesList[0]);
  return KEYSTONES[firstRune] || 'Recomendada';
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
        champId: championName.value,
        role: activeRole.value || null,
        elo: activeElo.value || null
      });
      if (builds && builds.length > 0) {
        const itemIds = typeof builds[0].items_json === 'string' ? JSON.parse(builds[0].items_json) : builds[0].items_json;
        recommendedItems.value = itemIds.map((id: any) => ({ item_id: id }));
        
        if (builds[0].runes_json) {
          recommendedRunes.value = typeof builds[0].runes_json === 'string' ? JSON.parse(builds[0].runes_json) : builds[0].runes_json;
        } else {
          recommendedRunes.value = null;
        }
      }
    } catch (e) { console.error(e); }
  }
};

watch(() => props.champion, async (newVal) => {
  if (newVal) {
    championName.value = newVal;
    isVisible.value = true;
    
    // Fetch runes for prop champion
    try {
      const builds: any[] = await invoke('get_recommended_builds_command', { 
        champId: newVal,
        role: activeRole.value || null,
        elo: activeElo.value || null
      });
      if (builds && builds.length > 0) {
        if (builds[0].runes_json) {
          recommendedRunes.value = typeof builds[0].runes_json === 'string' ? JSON.parse(builds[0].runes_json) : builds[0].runes_json;
        } else {
          recommendedRunes.value = null;
        }
      }
    } catch (e) { console.error("Failed to fetch runes for prop:", e); }
  }
}, { immediate: true });

watch(() => props.items, (newVal) => {
  if (newVal) {
    recommendedItems.value = newVal.map(id => typeof id === 'object' ? id : { item_id: id });
  }
}, { immediate: true });

watch(() => props.version, (newVal) => {
  if (newVal) currentVersion.value = newVal;
}, { immediate: true });

onMounted(async () => {
  if (props.champion) return; // Skip listeners if controlled by props

  const waitForDb = async () => {
    try {
      const ready = await invoke('is_db_ready');
      if (ready) {
        const versions: string[] = await invoke('get_ddragon_versions') as any;
        if (versions && versions.length > 0) currentVersion.value = versions[0];
      } else {
        setTimeout(waitForDb, 200);
      }
    } catch (e) {
      console.error("Failed to check DB readiness:", e);
      setTimeout(waitForDb, 200);
    }
  };

  waitForDb();

  await listen('lcu-update', (event: any) => {
    const data: any = event.payload;
    let role = null;
    let elo = data.elo || null;

    if (data.state && data.state.ChampSelect) {
      role = data.state.ChampSelect.role;
    }

    if (data.championName) {
      updateBuild(data.championName, role, elo);
    } else if (data.gameData) {
      // activePlayer.championName pode estar vazio em versões recentes do LoL.
      // Fallback: busca o campeão em allPlayers combinando summonerName com suporte
      // ao formato Riot ID (gameName#tagLine vs gameName).
      const activeSumm: string = data.gameData.activePlayer?.summonerName || '';
      const activeBase = activeSumm.split('#')[0]?.toLowerCase() || '';
      let champ: string = data.gameData.activePlayer?.championName || '';
      if (!champ && activeSumm) {
        const match = (data.gameData.allPlayers as any[] | undefined)?.find((p: any) => {
          const pName: string = p.summonerName || '';
          const pBase = pName.split('#')[0]?.toLowerCase() || '';
          return pName === activeSumm || (activeBase && pBase === activeBase);
        });
        champ = match?.championName || '';
      }
      if (champ) {
        updateBuild(champ, role, elo);
      }
    }
  });
});
</script>

<template>
  <div v-if="isVisible" class="build-row glass">
    <div class="champ-tag">
      <img :src="`https://ddragon.leagueoflegends.com/cdn/${currentVersion}/img/champion/${championName}.png`" class="champ-icon" />
      <span class="champ-name">{{ championName }}</span>
    </div>
    
    <div class="items-list">
      <div v-for="(item, idx) in recommendedItems" :key="idx" class="item-slot" :title="`Item ID: ${item.item_id}`">
        <img :src="`https://ddragon.leagueoflegends.com/cdn/${currentVersion}/img/item/${item.item_id}.png`" />
      </div>
    </div>

    <!-- Hextech Styled Recommended Runes Block -->
    <div v-if="recommendedRunes" class="runes-block">
      <div class="rune-tree-tag" :style="{ color: getRuneTree(recommendedRunes.primary_tree).color }">
        <span class="tree-name">{{ getRuneTree(recommendedRunes.primary_tree).name }}</span>
        <span class="keystone-name">{{ getKeystone(recommendedRunes.runes) }}</span>
      </div>
      <div class="rune-sub-tag">
        <span>+ {{ getRuneTree(recommendedRunes.secondary_tree).name }}</span>
      </div>
    </div>

    <div class="brand-tag">SPELL COACH IA</div>
  </div>
</template>

<style scoped>
.build-row {
  display: flex;
  align-items: center;
  width: 470px;
  height: 54px;
  background: rgba(5, 10, 20, 0.95);
  border: 1px solid rgba(200, 155, 60, 0.6);
  padding: 0 10px;
  border-radius: 4px;
  gap: 12px;
  position: relative;
  box-sizing: border-box;
}

.champ-tag {
  display: flex;
  align-items: center;
  gap: 8px;
  padding-right: 10px;
  border-right: 1px solid rgba(255, 255, 255, 0.1);
  min-width: 80px;
}

.champ-icon {
  width: 32px;
  height: 32px;
  border: 1px solid var(--accent-gold);
  border-radius: 2px;
}

.champ-name {
  font-size: 10px;
  font-weight: 800;
  color: var(--accent-gold);
  text-transform: uppercase;
}

.items-list {
  display: flex;
  gap: 4px;
}

.item-slot {
  width: 32px;
  height: 32px;
  border: 1px solid rgba(255, 255, 255, 0.2);
  background: black;
}

.item-slot img {
  width: 100%;
  height: 100%;
}

.runes-block {
  display: flex;
  flex-direction: column;
  justify-content: center;
  padding-left: 10px;
  border-left: 1px solid rgba(255, 255, 255, 0.1);
  height: 32px;
  gap: 2px;
  min-width: 110px;
}

.rune-tree-tag {
  display: flex;
  align-items: center;
  gap: 4px;
  font-size: 9px;
  font-weight: 800;
  text-transform: uppercase;
}

.tree-name {
  opacity: 0.8;
  font-size: 8px;
  background: rgba(255, 255, 255, 0.05);
  padding: 1px 3px;
  border-radius: 2px;
}

.keystone-name {
  color: #FFFFFF;
  font-weight: 700;
  text-transform: none;
  font-size: 10px;
  white-space: nowrap;
}

.rune-sub-tag {
  font-size: 8px;
  color: rgba(255, 255, 255, 0.5);
  font-weight: 600;
  text-transform: uppercase;
}

.brand-tag {
  position: absolute;
  right: 10px;
  bottom: 2px;
  font-size: 7px;
  font-weight: 800;
  color: var(--accent-gold);
  opacity: 0.4;
  letter-spacing: 1px;
}
</style>
