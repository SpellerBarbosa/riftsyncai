<script setup lang="ts">
import { ref, onMounted, computed } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import BuildWindow from "./BuildWindow.vue";
import Flashcard from "./Flashcard.vue";
import {
  TREE_STRUCTURES,
  SHARDS_ROWS,
  RUNE_COLORS,
  getFullUrl,
  getTreeHeaderIcon,
  normalizeRuneId
} from './runesData';


const appWindow = getCurrentWindow();
const isClosing = ref(false);

const minimize = () => appWindow.minimize();
const close = () => {
  isClosing.value = true;
  setTimeout(() => {
    appWindow.close();
  }, 300);
};

const rawData = ref<any>(null);
const loading = ref(false);
const error = ref<string | null>(null);

const fetchData = async (command: string, args: any = {}) => {
  loading.value = true;
  error.value = null;
  try {
    const res = await invoke(command, args) as any;
    rawData.value = res;
    
    // Auto-fill PUUID if we got an account
    if (command === 'get_account_by_riot_id' && res.puuid) {
      puuid.value = res.puuid;
    }
  } catch (e: any) {
    error.value = e.toString();
    rawData.value = null;
  } finally {
    loading.value = false;
  }
};

const fetchFullAccountData = async () => {
  if (!gameName.value || !tagLine.value) {
    error.value = "Preencha o Game Name e a Tag (ex: BR1)";
    return;
  }

  loading.value = true;
  error.value = null;
  try {
    // 1. Get Account (PUUID)
    const account: any = await invoke('get_account_by_riot_id', { 
      region: region.value, 
      gameName: gameName.value, 
      tagLine: tagLine.value 
    });
    
    puuid.value = account.puuid;
    
    // 2. Get Summoner and History in parallel
    const [summoner, history] = await Promise.all([
      invoke('get_summoner_by_puuid', { platform: platform.value, puuid: account.puuid }),
      invoke('get_match_history', { region: region.value, puuid: account.puuid, count: 5 })
    ]);

    rawData.value = {
      account,
      summoner,
      history
    };
  } catch (e: any) {
    error.value = e.toString();
    rawData.value = null;
  } finally {
    loading.value = false;
  }
};

/* 
const fetchMatchDetails = async (id: string) => {
  matchId.value = id;
  await fetchData('get_match_details', { region: region.value, matchId: id });
}; 
*/

// Default test values
const region = ref("americas");
const platform = ref("br1");
const gameName = ref("");
const tagLine = ref("");
const puuid = ref("");
// const matchId = ref("");

// Data Dragon
const ddVersion = ref("");
// const ddLang = ref("pt_BR");
// const ddChampId = ref("Aatrox");

const fetchLatestVersion = async () => {
  try {
    const versions: any = await invoke('get_ddragon_versions');
    if (versions && versions.length > 0) {
      ddVersion.value = versions[0];
    }
  } catch (e: any) {
    error.value = "Falha ao buscar versões: " + e.toString();
  }
};

// Matchup Explorer
const searchChamp = ref("");
const resolvedChampId = ref("");
const matchups = ref<any[]>([]);
const bestMatchups = ref<any[]>([]);
const worstMatchups = ref<any[]>([]);
const coreBuild = ref<number[]>([]);
const situationalItems = ref<any[]>([]);
const championRunes = ref<any>(null);
const matchupCount = ref(0);
const tacticalTips = ref({
  matchup_front: "Como lidar na rota?",
  matchup_back: "Selecione um campeão para carregar dicas táticas dinâmicas.",
  item_front: "Qual o pico de poder nos itens?",
  item_back: "Os itens ideais dependem do seu campeão selecionado."
});

const dbChampions = ref<any[]>([]);
const filteredChampions = computed(() => {
  if (!searchChamp.value) return dbChampions.value;
  return dbChampions.value.filter((c: any) => 
    c.name.toLowerCase().includes(searchChamp.value.toLowerCase()) || 
    c.id.toLowerCase().includes(searchChamp.value.toLowerCase())
  );
});

const fetchDbChampions = async () => {
  try {
    const list = await invoke('get_db_champions') as any[];
    dbChampions.value = list || [];
    if (list && list.length > 0 && !searchChamp.value) {
      searchChamp.value = list[0].id;
      await fetchChampionData();
    }
  } catch (e) {
    console.error("Erro ao buscar campeões do banco:", e);
  }
};

const selectChampion = async (id: string) => {
  searchChamp.value = id;
  await fetchChampionData();
};

const findRuneDetails = (id: number) => {
  if (!id) return null;
  const normalId = normalizeRuneId(id);
  for (const treeId in TREE_STRUCTURES) {
    const tree = TREE_STRUCTURES[treeId as unknown as number];
    const keystone = tree.keystones.find(k => k.id === normalId);
    if (keystone) return { ...keystone, treeId: tree.id, isKeystone: true };
    for (const row of tree.rows) {
      const rune = row.find(r => r.id === normalId);
      if (rune) return { ...rune, treeId: tree.id, isKeystone: false };
    }
  }
  // Fallback: rune not in local map — still show it with a generic icon
  return {
    id: normalId,
    name: `Runa ${id}`,
    icon: `https://raw.communitydragon.org/latest/plugins/rcp-be-lol-game-data/global/default/v1/perk-images/styles/domination/eyeballcollection/eyeballcollection.png`,
    treeId: 0,
    isKeystone: false,
    isFallback: true
  };
};

const findShardDetails = (id: number) => {
  for (const row of SHARDS_ROWS) {
    const shard = row.find(s => s.id === id);
    if (shard) return shard;
  }
  return null;
};

const isDbReady = ref(false);

const checkMatchupCount = async () => {
  try {
    matchupCount.value = await invoke('get_matchup_count_command') as number;
  } catch (e) {
    console.error(e);
  }
};

const forceMatchupSync = async () => {
  loading.value = true;
  error.value = null;
  try {
    await invoke('sync_matchups_command');
    await checkMatchupCount();
  } catch (e: any) {
    error.value = e.toString();
  } finally {
    loading.value = false;
  }
};

const syncStatusMsg = ref('');

const resetDb = async () => {
  if (!confirm('⚠️ RESET TOTAL: Todos os dados serão apagados e a aplicação fará uma resincronização completa. Continuar?')) return;
  loading.value = true;
  error.value = null;
  syncStatusMsg.value = 'Apagando banco de dados...';
  try {
    const msg = await invoke('reset_builds_and_runes_command') as string;
    syncStatusMsg.value = msg;
    matchupCount.value = 0;
    dbChampions.value = [];
    coreBuild.value = [];
    championRunes.value = null;
    rawData.value = null;
  } catch (e: any) {
    error.value = e.toString();
  } finally {
    loading.value = false;
  }
};

const syncRunes = async () => {
  loading.value = true;
  error.value = null;
  syncStatusMsg.value = 'Sincronizando runas do DDragon...';
  try {
    const res = await invoke('sync_and_validate_runes_command') as any;
    syncStatusMsg.value = `✅ Runas OK: ${res.fixed} de ${res.total_checked} corrigidos (v${res.version})`;
    await fetchDbChampions();
  } catch (e: any) {
    error.value = e.toString();
    syncStatusMsg.value = '';
  } finally {
    loading.value = false;
  }
};


const waitForDb = async () => {
  try {
    const ready = await invoke('is_db_ready');
    if (ready) {
      isDbReady.value = true;
      await checkMatchupCount();
      await fetchLatestVersion();
      await fetchDbChampions();
    } else {
      setTimeout(waitForDb, 200);
    }
  } catch (e) {
    console.error("Erro ao checar inicialização do banco:", e);
    setTimeout(waitForDb, 200);
  }
};

onMounted(() => {
  waitForDb();
  fetchLcuSummoner();
});

// Perfil & IA Coach State
const activeTab = ref<'matchup' | 'profile'>('matchup');
const riotPuuid = ref("");
const styleProfile = ref<any>(null);
const analyzedCoaching = ref<any>(null);
const isAnalyzingProfile = ref(false);

const fetchLcuSummoner = async () => {
  try {
    const summoner: any = await invoke("get_current_summoner");
    if (summoner && summoner.puuid) {
      riotPuuid.value = summoner.puuid;
      gameName.value = summoner.gameName || summoner.displayName || "";
      tagLine.value = summoner.tagLine || "";
      
      // Persist in SQLite
      await invoke("save_lcu_summoner_command", {
        puuid: summoner.puuid,
        gameName: gameName.value,
        tagLine: tagLine.value
      });
    } else {
      await loadSavedSummoner();
    }
  } catch (e) {
    console.warn("LCU not connected or no summoner data:", e);
    await loadSavedSummoner();
  }
};

const loadSavedSummoner = async () => {
  try {
    const saved: any = await invoke("get_saved_summoner_command");
    if (saved) {
      riotPuuid.value = saved.puuid || "";
      gameName.value = saved.game_name || "";
      tagLine.value = saved.tag_line || "";
    }
  } catch (err) {
    console.error("Erro ao ler summoner salvo do banco:", err);
  }
};

const fetchPlayerStyleAnalysis = async () => {
  if (!riotPuuid.value) {
    error.value = "Insira um PUUID válido ou conecte o cliente do LoL.";
    return;
  }
  isAnalyzingProfile.value = true;
  error.value = null;
  styleProfile.value = null;
  analyzedCoaching.value = null;
  
  try {
    const profile = await invoke("get_player_style_analysis", {
      puuid: riotPuuid.value,
      region: region.value,
      count: 15
    }) as any;
    
    styleProfile.value = profile;
    
    if (profile.coaching_tips) {
      try {
        analyzedCoaching.value = JSON.parse(profile.coaching_tips);
      } catch (err) {
        console.error("Erro ao fazer parse dos treinos da IA:", err);
      }
    }

    // Persist details
    try {
      await invoke("save_lcu_summoner_command", {
        puuid: riotPuuid.value,
        gameName: gameName.value || "",
        tagLine: tagLine.value || ""
      });
    } catch (dbErr) {
      console.warn("Falha ao salvar dados de summoner do perfil:", dbErr);
    }
  } catch (e: any) {
    error.value = e.toString();
  } finally {
    isAnalyzingProfile.value = false;
  }
};

const fetchChampionData = async () => {
  if (!searchChamp.value) return;
  loading.value = true;
  error.value = null;
  matchups.value = [];
  coreBuild.value = [];
  situationalItems.value = [];
  championRunes.value = null;
  
  try {
    // 1. Fetch Matchups
    const matchRes = await invoke('get_champion_matchups_command', { champId: searchChamp.value }) as any;
    resolvedChampId.value = matchRes.champion_id;
    const list = matchRes.matchups;
    matchups.value = list;

    if (list.length === 0) {
      console.warn(`Nenhum matchup encontrado para "${matchRes.champion_id}".`);
    }

    bestMatchups.value = list.length > 0 ? [...list].sort((a: any, b: any) => b.win_rate - a.win_rate).slice(0, 5) : [];
    worstMatchups.value = list.length > 0 ? [...list].sort((a: any, b: any) => a.win_rate - b.win_rate).slice(0, 5) : [];

    // 2. Fetch Builds
    const buildRes = await invoke('get_champion_build_command', { champId: resolvedChampId.value }) as any[];
    if (buildRes.length > 0) {
      coreBuild.value = buildRes[0]; // Take first role's core items
    }

    // 2.5 Fetch Runes
    try {
      const runesRes = await invoke('get_champion_runes_command', { champId: resolvedChampId.value }) as any;
      championRunes.value = runesRes;
    } catch (err) {
      console.warn("Nenhuma runa recomendada no banco:", err);
      championRunes.value = null;
    }

    // 3. Fetch Situational Items
    const sitRes = await invoke('get_situational_items_command', { champId: resolvedChampId.value }) as any[];
    situationalItems.value = sitRes;
    
    // 4. Fetch Dynamic Tactical Tips
    try {
      const tipsRes = await invoke('get_tactical_tips_command', { champId: resolvedChampId.value }) as any;
      tacticalTips.value = tipsRes;
    } catch (err) {
      console.error("Erro ao carregar dicas táticas:", err);
    }
    
    rawData.value = { matchups: matchRes, builds: buildRes, situational: sitRes, runes: championRunes.value };
  } catch (e: any) {
    error.value = e.toString();
  } finally {
    loading.value = false;
  }
};
</script>

<template>
  <div :class="['data-viewer', 'glass', { 'fade-out': isClosing }]">
    <aside class="sidebar">
      <header data-tauri-drag-region class="sidebar-header">
        <h3 data-tauri-drag-region>Data Explorer</h3>
        <div class="window-controls">
          <button class="control-btn" @click="minimize">_</button>
          <button class="control-btn close" @click="close">×</button>
        </div>
      </header>

      <!-- Tab Switcher -->
      <div class="tab-switcher">
        <button :class="{ active: activeTab === 'matchup' }" @click="activeTab = 'matchup'">⚔️ Confrontos</button>
        <button :class="{ active: activeTab === 'profile' }" @click="activeTab = 'profile'">👤 Estilo & IA</button>
      </div>

      <!-- Tab 1: Matchup Explorer Sidebar controls -->
      <div v-if="activeTab === 'matchup'" class="sidebar-tab-content">
        <!-- Primary Action: Search -->
        <section class="sidebar-section">
          <h3>Explorar Campeão</h3>
          <div class="input-group">
            <input v-model="searchChamp" placeholder="Buscar campeão (ex: Jinx...)" @keyup.enter="fetchChampionData" />
            <button class="btn-api-gold" @click="fetchChampionData">Analisar</button>
          </div>
          <div class="champion-mini-list mt-s" v-if="filteredChampions.length > 0">
            <button 
              v-for="c in filteredChampions.slice(0, 15)" 
              :key="c.id" 
              :class="['champ-mini-btn', { active: searchChamp && searchChamp.toLowerCase() === c.id.toLowerCase() }]"
              @click="selectChampion(c.id)"
            >
              <img :src="`https://ddragon.leagueoflegends.com/cdn/${ddVersion || '16.10.1'}/img/champion/${c.id}.png`" class="champ-avatar-micro" />
              <span>{{ c.name }}</span>
            </button>
          </div>
        </section>

        <!-- Sync Status -->
        <section class="sidebar-section mt">
          <h3>Banco de Dados</h3>
          <div class="sync-card">
            <span class="count">{{ matchupCount }} Registros</span>
            <span v-if="syncStatusMsg" class="sync-status-msg">{{ syncStatusMsg }}</span>
            <button class="btn-sync" @click="forceMatchupSync" :disabled="loading">
              <span v-if="loading">Sincronizando...</span>
              <span v-else>Forçar Sincronização</span>
            </button>
            <button class="btn-sync rune-btn" @click="syncRunes" :disabled="loading" title="Sincroniza runas do DDragon e valida 4+2+3">
              🔮 Validar Runas (4+2+3)
            </button>
            <button class="btn-reset" @click="resetDb" :disabled="loading" title="Apaga todos os dados e força resincronização completa">
              🗑️ Reset Total do Banco
            </button>
          </div>
        </section>
      </div>

      <!-- Tab 2: Profile Dashboard Sidebar controls -->
      <div v-else-if="activeTab === 'profile'" class="sidebar-tab-content">
        <section class="sidebar-section">
          <h3>Análise de Estilo</h3>
          <div class="input-group">
            <label class="label-field">Riot PUUID</label>
            <input v-model="riotPuuid" placeholder="Cole seu PUUID ou conecte o LoL" />
            
            <label class="label-field">Região (Partidas V5)</label>
            <select v-model="region" class="select-field">
              <option value="americas">Americas (BR, NA, LA)</option>
              <option value="europe">Europe</option>
              <option value="asia">Asia</option>
            </select>
            
            <button class="btn-api-gold mt-s" @click="fetchPlayerStyleAnalysis">
              <span v-if="isAnalyzingProfile">Analisando...</span>
              <span v-else>💡 Analisar Estilo</span>
            </button>
          </div>
          
          <div class="quick-profile-info mt" v-if="styleProfile">
            <div class="profile-badge-micro">
              Estilo: <strong>{{ styleProfile.style_tag }}</strong>
            </div>
            <div class="profile-games-count">
              Histórico: {{ styleProfile.total_games }} jogos analisados
            </div>
          </div>
        </section>
      </div>

      <!-- Collapsible Debug Tools -->
      <section class="sidebar-section mt debug-tools">
        <h3>Ferramentas de Debug</h3>
        <details>
          <summary>LCU / LCA (Local)</summary>
          <div class="debug-grid">
            <button @click="fetchData('get_lcu_status')">Status</button>
            <button @click="fetchData('get_current_summoner')">Summoner</button>
            <button @click="fetchData('get_all_game_data')">LCA Data</button>
          </div>
        </details>
        
        <details class="mt-s">
          <summary>Riot Web API</summary>
          <div class="input-group">
            <input v-model="gameName" placeholder="Game Name" />
            <input v-model="tagLine" placeholder="Tag (ex: BR1)" />
            <button class="btn-api" @click="fetchFullAccountData">Full Sync</button>
          </div>
        </details>

        <details class="mt-s">
          <summary>Data Dragon</summary>
          <div class="debug-grid">
            <button @click="fetchLatestVersion">Update Ver</button>
            <input v-model="ddVersion" placeholder="Version" />
            <button @click="fetchData('get_ddragon_champions')">Champs</button>
          </div>
        </details>
      </section>

      <div class="sidebar-footer">
        <span>v{{ ddVersion || '...' }}</span>
        <span>Spell Coach IA</span>
      </div>
    </aside>

    <main class="content">
      <div v-if="loading" class="loader">Buscando dados...</div>
      <div v-else-if="error" class="error-box">Erro: {{ error }}</div>
      
      <!-- Tab 1 Main View: Champion Explorer -->
      <div v-else-if="activeTab === 'matchup'" class="tab-view-wrapper">
        <div v-if="resolvedChampId" class="matchup-explorer-view animate-fade">
          <div class="explorer-header">
            <div class="main-champ">
              <img :src="`https://ddragon.leagueoflegends.com/cdn/${ddVersion || '16.10.1'}/img/champion/${resolvedChampId}.png`" />
              <div class="title-stack">
                <h2>{{ resolvedChampId }}</h2>
                <span class="subtitle">Análise Tática de Matchups e Builds</span>
              </div>
            </div>
            <div class="explorer-stats">
              <span>{{ matchups.length }} Matchups • {{ coreBuild.length }} Itens Core</span>
            </div>
          </div>

          <!-- Build Section -->
          <section class="build-summary-section">
            <div class="section-header">
              <span class="icon">⚔️</span>
              <h3>Build Recomendada (Core)</h3>
            </div>
            <div class="item-row">
              <div v-for="id in coreBuild" :key="id" class="item-icon-wrapper" :title="`Item ID: ${id}`">
                <img :src="`https://ddragon.leagueoflegends.com/cdn/${ddVersion || '16.10.1'}/img/item/${id}.png`" @error="(e:any) => e.target.style.display='none'" />
              </div>
            </div>
          </section>

          <!-- Runes Section -->
          <section v-if="championRunes" class="runes-summary-section mt">
            <div class="section-header">
              <span class="icon">🔮</span>
              <h3>Runas Recomendadas</h3>
            </div>
            <div class="runes-container-grid">
              <!-- Primary Tree -->
              <div class="rune-tree-box primary" :style="{ '--tree-accent': RUNE_COLORS[championRunes.primary_tree] || '#C89B3C' }">
                <div class="tree-header-row">
                  <img :src="getTreeHeaderIcon(championRunes.primary_tree)" class="tree-header-icon" />
                  <span class="tree-title">{{ TREE_STRUCTURES[championRunes.primary_tree]?.name || 'Primária' }}</span>
                </div>
                <div class="runes-chips-list">
                  <!-- Keystone -->
                  <div class="rune-chip keystone" v-if="championRunes.runes[0] > 0">
                    <img :src="getFullUrl(findRuneDetails(championRunes.runes[0])?.icon || '')" class="rune-icon" />
                    <div class="rune-info-text">
                      <span class="rune-type">Runa Principal</span>
                      <span class="rune-name">{{ findRuneDetails(championRunes.runes[0])?.name }}</span>
                    </div>
                  </div>
                  <!-- Sub-runes (indices 1, 2, 3) -->
                  <template v-for="runeId in championRunes.runes.slice(1, 4)" :key="runeId">
                    <div class="rune-chip" v-if="runeId > 0">
                      <img :src="getFullUrl(findRuneDetails(runeId)?.icon || '')" class="rune-icon sm" />
                      <span class="rune-name">{{ findRuneDetails(runeId)?.name }}</span>
                    </div>
                  </template>
                </div>
              </div>

              <!-- Secondary Tree & Shards -->
              <div class="rune-tree-box secondary" :style="{ '--tree-accent': RUNE_COLORS[championRunes.secondary_tree] || '#a09b8c' }">
                <div class="tree-header-row">
                  <img :src="getTreeHeaderIcon(championRunes.secondary_tree)" class="tree-header-icon sm" />
                  <span class="tree-title">{{ TREE_STRUCTURES[championRunes.secondary_tree]?.name || 'Secundária' }}</span>
                </div>
                <div class="runes-chips-list">
                  <template v-for="runeId in championRunes.runes.slice(4, 6)" :key="runeId">
                    <div class="rune-chip" v-if="runeId > 0">
                      <img :src="getFullUrl(findRuneDetails(runeId)?.icon || '')" class="rune-icon sm" />
                      <span class="rune-name">{{ findRuneDetails(runeId)?.name }}</span>
                    </div>
                  </template>
                </div>

                <!-- Shards (Attributes) -->
                <div class="shards-summary-box mt-s" v-if="championRunes.shards && championRunes.shards.length > 0">
                  <span class="shards-title">Atributos</span>
                  <div class="shards-row">
                    <div 
                      v-for="(shardId, idx) in championRunes.shards" 
                      :key="idx" 
                      class="shard-chip"
                      v-show="findShardDetails(shardId)"
                      :title="findShardDetails(shardId)?.name"
                    >
                      <img :src="getFullUrl(findShardDetails(shardId)?.icon || '')" class="shard-icon" />
                    </div>
                  </div>
                </div>
              </div>
            </div>
          </section>

          <div class="matchup-grids">
            <section class="matchup-column best">
              <h3><span class="icon">↑</span> Melhores Confrontos</h3>
              <div class="matchup-card-list">
                <div v-for="m in bestMatchups" :key="m.opponent_id" class="matchup-card">
                  <img :src="`https://ddragon.leagueoflegends.com/cdn/${ddVersion || '16.10.1'}/img/champion/${m.opponent_id}.png`" />
                  <div class="card-info">
                    <span class="name">{{ m.opponent_id }}</span>
                    <div class="win-bar-bg">
                      <div class="win-bar" :style="{ width: (m.win_rate * 100) + '%', background: '#4eff9b' }"></div>
                    </div>
                    <div class="card-stats">
                      <span class="percentage">{{ (m.win_rate * 100).toFixed(1) }}% WR</span>
                      <span class="win-loss" v-if="m.games_count">V: {{ m.wins_count || 0 }} | D: {{ (m.games_count || 0) - (m.wins_count || 0) }}</span>
                    </div>
                  </div>
                </div>
              </div>
            </section>

            <section class="matchup-column worst">
              <h3><span class="icon">↓</span> Piores Confrontos (Counters)</h3>
              <div class="matchup-card-list">
                <div v-for="m in worstMatchups" :key="m.opponent_id" class="matchup-card">
                  <img :src="`https://ddragon.leagueoflegends.com/cdn/${ddVersion || '16.10.1'}/img/champion/${m.opponent_id}.png`" />
                  <div class="card-info">
                    <span class="name">{{ m.opponent_id }}</span>
                    <div class="win-bar-bg">
                      <div class="win-bar" :style="{ width: ((1.0 - m.win_rate) * 100) + '%', background: '#ff4e4e' }"></div>
                    </div>
                    <div class="card-stats">
                      <span class="percentage danger">{{ ((1.0 - m.win_rate) * 100).toFixed(1) }}% WR (Op.)</span>
                      <span class="win-loss" v-if="m.games_count">V: {{ m.wins_count || 0 }} | D: {{ (m.games_count || 0) - (m.wins_count || 0) }}</span>
                    </div>
                  </div>
                </div>
              </div>
            </section>
          </div>

          <!-- Situational Items -->
          <section v-if="situationalItems.length > 0" class="situational-section">
            <div class="section-header">
              <span class="icon">💎</span>
              <h3>Itens Situacionais e Transição</h3>
            </div>
            <div class="situational-grid">
              <div v-for="item in situationalItems.slice(0, 8)" :key="item.item_id" class="sit-card">
                <img :src="`https://ddragon.leagueoflegends.com/cdn/${ddVersion || '16.10.1'}/img/item/${item.item_id}.png`" />
                <div class="sit-info">
                  <span class="slot">Slot: {{ item.slot_type }}</span>
                  <span class="wr">{{ (item.win_rate * 100).toFixed(1) }}% WR</span>
                </div>
              </div>
            </div>
          </section>

          <!-- Overlay Preview Section -->
          <section class="preview-section mt">
            <div class="section-header">
              <span class="icon">👁️</span>
              <h3>Simulação do Overlay (In-Game)</h3>
            </div>
            <div class="preview-container">
              <BuildWindow :champion="resolvedChampId" :items="coreBuild" :version="ddVersion" />
            </div>
          </section>

          <!-- Flashcard Tactical Tips Section -->
          <section class="flashcard-section mt">
            <div class="section-header">
              <span class="icon">🃏</span>
              <h3>Dicas Táticas (Flashcards)</h3>
            </div>
            <div class="flashcards-grid">
              <Flashcard 
                title="Dica de Matchup" 
                :frontText="tacticalTips.matchup_front" 
                :backText="tacticalTips.matchup_back"
                rarity="epic"
              />
              <Flashcard 
                title="Dica de Item" 
                :frontText="tacticalTips.item_front" 
                :backText="tacticalTips.item_back"
                rarity="legendary"
              />
            </div>
          </section>
        </div>
        <div v-else class="placeholder">Selecione uma chamada ou busque um campeão para visualizar os dados.</div>
      </div>

      <!-- Tab 2 Main View: Player Style Dashboard -->
      <div v-else-if="activeTab === 'profile'" class="tab-view-wrapper animate-fade">
        <div v-if="isAnalyzingProfile" class="profile-loader-box">
          <div class="hextech-spinner"></div>
          <p class="mt">Persistindo dados no SQLite e gerando auditoria tática com IA...</p>
        </div>

        <div v-else-if="styleProfile" class="player-profile-view">
          <div class="profile-dashboard-header">
            <div class="hextech-profile-card">
              <div class="style-badge-frame">
                <span class="style-sparkle">⚡</span>
                <span class="badge-title">{{ styleProfile.style_tag }}</span>
              </div>
              <p class="style-description">{{ styleProfile.style_description }}</p>
            </div>
          </div>

          <!-- Stat Bars Grid -->
          <section class="profile-metrics-card mt">
            <div class="section-header">
              <span class="icon">📈</span>
              <h3>Métricas de Desempenho e Benchmarks</h3>
            </div>
            <div class="metrics-grid">
              <!-- CS/min -->
              <div class="metric-item">
                <div class="metric-label">
                  <span>Farm por Minuto</span>
                  <strong>{{ styleProfile.avg_cs_per_min.toFixed(1) }} CS/m</strong>
                </div>
                <div class="metric-progress-bg">
                  <div class="metric-progress-bar gold" :style="{ width: Math.min(100, (styleProfile.avg_cs_per_min / 9.0) * 100) + '%' }"></div>
                </div>
                <span class="benchmark">Meta Challenger: 8.5+ CS/min</span>
              </div>

              <!-- Vision Score -->
              <div class="metric-item">
                <div class="metric-label">
                  <span>Visão por Minuto</span>
                  <strong>{{ styleProfile.avg_vision_score_per_min.toFixed(2) }} Score/m</strong>
                </div>
                <div class="metric-progress-bg">
                  <div class="metric-progress-bar teal" :style="{ width: Math.min(100, (styleProfile.avg_vision_score_per_min / 1.5) * 100) + '%' }"></div>
                </div>
                <span class="benchmark">Meta Challenger: 1.2+ Score/min</span>
              </div>

              <!-- KDA -->
              <div class="metric-item">
                <div class="metric-label">
                  <span>KDA Pessoal</span>
                  <strong>{{ styleProfile.avg_kda.toFixed(2) }} : 1</strong>
                </div>
                <div class="metric-progress-bg">
                  <div class="metric-progress-bar blue" :style="{ width: Math.min(100, (styleProfile.avg_kda / 4.0) * 100) + '%' }"></div>
                </div>
                <span class="benchmark">Meta Challenger: 3.5+ KDA</span>
              </div>

              <!-- Deaths -->
              <div class="metric-item">
                <div class="metric-label">
                  <span>Média de Mortes</span>
                  <strong>{{ styleProfile.avg_deaths.toFixed(1) }} Mortes</strong>
                </div>
                <div class="metric-progress-bg">
                  <div class="metric-progress-bar red" :style="{ width: Math.min(100, (styleProfile.avg_deaths / 10.0) * 100) + '%' }"></div>
                </div>
                <span class="benchmark text-red">Meta Challenger: &lt; 4.5 mortes</span>
              </div>
            </div>
          </section>

          <!-- AI Drills & Mistakes Grid -->
          <div class="coaching-columns mt" v-if="analyzedCoaching">
            <section class="coaching-card mistakes">
              <div class="section-header text-red">
                <span class="icon">⚠️</span>
                <h3>Erros Recorrentes Detectados</h3>
              </div>
              <ul>
                <li v-for="(err, idx) in analyzedCoaching.erros" :key="idx">
                  <span class="bullet">✖</span> {{ err }}
                </li>
              </ul>
            </section>

            <section class="coaching-card drills">
              <div class="section-header text-gold">
                <span class="icon">🎯</span>
                <h3>Treinos Práticos do Challenger Coach</h3>
              </div>
              <ul>
                <li v-for="(drill, idx) in analyzedCoaching.treinos" :key="idx" class="drill-item">
                  <div class="drill-checkbox">✓</div>
                  <div class="drill-text">{{ drill }}</div>
                </li>
              </ul>
            </section>
          </div>

          <!-- Historic Log List -->
          <section class="historic-matches-card mt">
            <div class="section-header">
              <span class="icon">💾</span>
              <h3>Histórico Persistido no SQLite</h3>
            </div>
            <div class="historic-match-grid">
              <div v-for="m in styleProfile.recent_matches" :key="m.match_id" class="historic-match-row" :class="m.win ? 'win' : 'lose'">
                <div class="match-champ-info">
                  <img :src="`https://ddragon.leagueoflegends.com/cdn/${ddVersion || '16.10.1'}/img/champion/${m.champion_name}.png`" @error="(e:any) => e.target.src='https://raw.communitydragon.org/latest/plugins/rcp-be-lol-game-data/global/default/v1/champion-icons/266.png'" />
                  <div class="details">
                    <span class="champ-name">{{ m.champion_name }}</span>
                    <span class="position-tag">{{ m.position }}</span>
                  </div>
                </div>
                <div class="match-kda">
                  <span class="score">{{ m.kills }} / {{ m.deaths }} / {{ m.assists }}</span>
                  <span class="kda-ratio">{{ ((m.kills + m.assists) / Math.max(1, m.deaths)).toFixed(2) }} KDA</span>
                </div>
                <div class="match-performance">
                  <span>🌾 {{ m.cs_per_min.toFixed(1) }} CS/m</span>
                  <span>👁️ {{ m.vision_score_per_min.toFixed(2) }} Vis/m</span>
                </div>
                <div class="match-status-tag">
                  {{ m.win ? 'VITÓRIA' : 'DERROTA' }}
                </div>
              </div>
            </div>
          </section>
        </div>

        <div v-else class="placeholder-profile animate-fade">
          <div class="hextech-crest">⚜️</div>
          <h2>Conecte e Carregue seu Estilo</h2>
          <p>Para gerar sua auditoria tática personalizada e descobrir erros recorrentes, clique no botão <strong>"Analisar Estilo"</strong> no menu lateral.</p>
        </div>
      </div>
    </main>
  </div>
</template>

<style scoped>
.data-viewer {
  display: flex;
  width: 100vw;
  height: 100vh;
  background: #010a13;
  color: #f0e6d2;
  animation: slideUp 0.4s ease-out forwards;
  transition: all 0.3s ease;
}

.data-viewer.fade-out {
  opacity: 0;
  transform: scale(0.95);
}

@keyframes slideUp {
  from { opacity: 0; transform: translateY(20px); }
  to { opacity: 1; transform: translateY(0); }
}

.sidebar {
  width: 280px;
  background: rgba(30, 35, 40, 0.5);
  border-right: 1px solid var(--accent-gold);
  padding: 16px;
  display: flex;
  flex-direction: column;
  gap: 8px;
  overflow-y: auto;
}

.sidebar-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 20px;
  border-bottom: 1px solid var(--glass-border);
  padding-bottom: 10px;
  cursor: move;
}

.window-controls {
  display: flex;
  gap: 4px;
}

.control-btn {
  background: transparent;
  border: none;
  color: var(--accent-gold);
  font-size: 14px;
  cursor: pointer;
  padding: 2px 6px;
}

.control-btn:hover {
  background: rgba(255, 255, 255, 0.1);
}

.control-btn.close:hover {
  background: #ff4e4e;
  color: white;
}

h3 {
  font-size: 12px;
  color: var(--accent-gold);
  text-transform: uppercase;
  margin-bottom: 8px;
}

.mt { margin-top: 24px; }
.mt-s { margin-top: 12px; }

button {
  background: rgba(255, 255, 255, 0.05);
  border: 1px solid rgba(200, 155, 60, 0.3);
  color: #f0e6d2;
  padding: 8px;
  text-align: left;
  cursor: pointer;
  font-size: 12px;
  transition: all 0.2s;
}

button:hover {
  background: rgba(200, 155, 60, 0.1);
  border-color: var(--accent-gold);
}

.input-group {
  display: flex;
  flex-direction: column;
  gap: 4px;
}

input {
  background: #1e2328;
  border: 1px solid var(--glass-border);
  color: white;
  padding: 6px;
  font-size: 11px;
}

.btn-api {
  background: var(--accent-blue);
  color: white;
  border: none;
  font-weight: bold;
}

.btn-api-gold {
  background: linear-gradient(to bottom, #c89b3c 0%, #785a28 100%);
  color: #1e2328;
  border: 1px solid #f0e6d2;
  font-weight: bold;
  padding: 8px;
  cursor: pointer;
  text-transform: uppercase;
  font-size: 10px;
}

.match-list {
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.btn-match {
  background: rgba(0, 0, 0, 0.2);
  border: 1px solid var(--glass-border);
  color: #4eff9b;
  font-family: monospace;
  font-size: 10px;
  text-align: center;
}

.btn-match:hover {
  background: var(--accent-blue);
  color: white;
}

.content {
  flex: 1;
  padding: 20px;
  overflow: auto;
  background: #010a13;
}

.json-viewer {
  font-family: 'Courier New', Courier, monospace;
  font-size: 12px;
  color: #4eff9b;
  white-space: pre-wrap;
  background: rgba(0, 0, 0, 0.3);
  padding: 12px;
  border-radius: 4px;
}

.loader { color: var(--accent-blue); }
.error-box { color: #ff4e4e; background: rgba(255, 78, 78, 0.1); padding: 12px; border: 1px solid #ff4e4e; }
.placeholder { color: rgba(255, 255, 255, 0.3); display: flex; align-items: center; justify-content: center; height: 100%; font-style: italic; }

.sidebar-section {
  display: flex;
  flex-direction: column;
}

.sync-card {
  background: rgba(0,0,0,0.3);
  border: 1px solid rgba(255,255,255,0.05);
  border-radius: 4px;
  padding: 10px;
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.sync-card .count {
  font-size: 10px;
  color: var(--text-secondary);
  text-align: center;
}

.btn-sync {
  background: var(--accent-blue);
  color: white;
  border: none;
  padding: 6px;
  border-radius: 2px;
  font-weight: 800;
  font-size: 9px;
  text-transform: uppercase;
  text-align: center;
}

.debug-tools details {
  background: rgba(255,255,255,0.02);
  border-radius: 4px;
  overflow: hidden;
}

.debug-tools summary {
  font-size: 10px;
  color: var(--text-secondary);
  padding: 6px 10px;
  cursor: pointer;
  user-select: none;
  border-bottom: 1px solid rgba(255,255,255,0.05);
}

.debug-tools summary:hover {
  background: rgba(255,255,255,0.05);
  color: white;
}

.debug-grid {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 4px;
  padding: 8px;
}

.debug-grid button {
  font-size: 9px;
  padding: 4px;
}

.sidebar-footer {
  margin-top: auto;
  display: flex;
  justify-content: space-between;
  font-size: 9px;
  opacity: 0.3;
  padding-top: 20px;
}

/* Matchup Explorer Styles */
.matchup-explorer-view {
  display: flex;
  flex-direction: column;
  gap: 24px;
  animation: fadeIn 0.3s ease-out;
  max-width: 900px;
  margin: 0 auto;
}

.explorer-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 16px;
  background: linear-gradient(90deg, rgba(200, 155, 60, 0.1) 0%, transparent 100%);
  border-left: 4px solid var(--accent-gold);
}

.main-champ {
  display: flex;
  align-items: center;
  gap: 16px;
}

.main-champ img {
  width: 64px;
  height: 64px;
  border: 2px solid var(--accent-gold);
  border-radius: 4px;
}

.main-champ h2 {
  font-family: 'Beaufort for LOL', serif;
  color: var(--accent-gold);
  text-transform: uppercase;
  letter-spacing: 2px;
  margin: 0;
}

.title-stack {
  display: flex;
  flex-direction: column;
}

.subtitle {
  font-size: 10px;
  color: var(--text-secondary);
  text-transform: uppercase;
  letter-spacing: 1px;
}

/* Build Summary Section */
.build-summary-section, .situational-section {
  background: rgba(255, 255, 255, 0.02);
  padding: 16px;
  border-radius: 4px;
  border: 1px solid rgba(255, 255, 255, 0.05);
}

.section-header {
  display: flex;
  align-items: center;
  gap: 10px;
  margin-bottom: 12px;
}

.section-header h3 {
  font-size: 12px;
  text-transform: uppercase;
  color: var(--accent-gold);
  margin: 0;
}

.item-row {
  display: flex;
  gap: 8px;
}

.item-icon-wrapper img {
  width: 36px;
  height: 36px;
  border: 1px solid var(--accent-gold-glow);
  border-radius: 4px;
  background: rgba(0,0,0,0.4);
}

.matchup-grids {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 20px;
}

.matchup-card-list {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.matchup-card {
  display: flex;
  align-items: center;
  gap: 12px;
  background: rgba(0,0,0,0.25);
  padding: 8px;
  border-radius: 4px;
  border: 1px solid rgba(255,255,255,0.03);
}

.matchup-card img {
  width: 40px;
  height: 40px;
  border-radius: 4px;
}

.card-info {
  flex: 1;
}

.card-info .name {
  display: block;
  font-size: 11px;
  font-weight: 700;
  margin-bottom: 4px;
}

.win-bar-bg {
  height: 4px;
  background: rgba(255,255,255,0.05);
  border-radius: 2px;
  overflow: hidden;
  margin-bottom: 4px;
}

.win-bar {
  height: 100%;
  border-radius: 2px;
}

.card-stats {
  display: flex;
  justify-content: space-between;
  align-items: center;
  font-size: 10px;
  font-weight: 800;
  margin-top: 2px;
}

.percentage {
  font-size: 10px;
  font-weight: 800;
  color: #4eff9b;
}

.percentage.danger {
  color: #ff4e4e;
}

.win-loss {
  color: #a09b8c;
  font-family: monospace;
  font-size: 9px;
  opacity: 0.8;
}

.situational-grid {
  display: grid;
  grid-template-columns: repeat(4, 1fr);
  gap: 12px;
}

.sit-card {
  display: flex;
  align-items: center;
  gap: 8px;
  background: rgba(0,0,0,0.2);
  padding: 6px;
  border-radius: 4px;
}

.sit-card img {
  width: 32px;
  height: 32px;
  border-radius: 2px;
}

.sit-info {
  display: flex;
  flex-direction: column;
}

.sit-info .slot {
  font-size: 8px;
  color: var(--text-secondary);
}

.sit-info .wr {
  font-size: 10px;
  font-weight: 800;
  color: #4eff9b;
}

/* Preview Section */
.preview-section {
  border-top: 1px solid rgba(255, 255, 255, 0.1);
  padding-top: 24px;
}

.preview-container {
  display: flex;
  justify-content: center;
  background: url('https://raw.communitydragon.org/latest/plugins/rcp-be-lol-game-data/global/default/v1/champion-icons/266.png') center/cover;
  padding: 40px;
  border-radius: 8px;
  position: relative;
  overflow: hidden;
}

.preview-container::before {
  content: '';
  position: absolute;
  inset: 0;
  background: rgba(0,0,0,0.6);
  backdrop-filter: blur(4px);
}

.preview-container > * {
  position: relative;
  z-index: 1;
  width: 350px;
}

.flashcards-grid {
  display: flex;
  flex-wrap: wrap;
  gap: 20px;
  justify-content: center;
  padding: 20px 0;
}

.flashcard-section {
  border-top: 1px solid rgba(255, 255, 255, 0.1);
  padding-top: 24px;
  margin-bottom: 40px;
}

@keyframes fadeIn {
  from { opacity: 0; transform: translateY(10px); }
  to { opacity: 1; transform: translateY(0); }
}

/* Tab Switcher and Controls */
.tab-switcher {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 8px;
  margin-bottom: 16px;
  background: rgba(0, 0, 0, 0.2);
  padding: 4px;
  border-radius: 4px;
  border: 1px solid rgba(200, 155, 60, 0.2);
}

.tab-switcher button {
  background: transparent;
  border: none;
  color: #a09b8c;
  padding: 6px;
  text-align: center;
  font-size: 11px;
  font-weight: 800;
  cursor: pointer;
  border-radius: 3px;
  transition: all 0.2s ease;
}

.tab-switcher button:hover {
  background: rgba(255, 255, 255, 0.05);
  color: #f0e6d2;
}

.tab-switcher button.active {
  background: #c89b3c;
  color: #010a13;
  box-shadow: 0 0 8px rgba(200, 155, 60, 0.4);
}

.sidebar-tab-content {
  display: flex;
  flex-direction: column;
  gap: 12px;
  animation: fadeIn 0.3s ease-out forwards;
}

.label-field {
  font-size: 9px;
  color: #c89b3c;
  text-transform: uppercase;
  font-weight: 800;
  letter-spacing: 0.5px;
  margin-top: 6px;
}

.select-field {
  background: #1e2328;
  border: 1px solid var(--glass-border);
  color: white;
  padding: 6px;
  font-size: 11px;
  cursor: pointer;
  outline: none;
  width: 100%;
}

.select-field:focus {
  border-color: #c89b3c;
}

.quick-profile-info {
  background: rgba(0, 0, 0, 0.2);
  border: 1px solid rgba(200, 155, 60, 0.15);
  padding: 8px;
  border-radius: 4px;
  font-size: 10px;
}

.quick-profile-info strong {
  color: #c89b3c;
}

/* Animations */
.animate-fade {
  animation: fadeIn 0.4s ease-out forwards;
}

/* Profile Tab Styles */
.tab-view-wrapper {
  width: 100%;
  height: 100%;
}

.profile-loader-box {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  height: 80%;
  color: #c89b3c;
  font-weight: 800;
  font-size: 13px;
  text-align: center;
}

.hextech-spinner {
  width: 48px;
  height: 48px;
  border: 3px solid rgba(200, 155, 60, 0.1);
  border-top-color: #c89b3c;
  border-radius: 50%;
  animation: spin 1s linear infinite;
  box-shadow: 0 0 10px rgba(200, 155, 60, 0.2);
}

@keyframes spin {
  to { transform: rotate(360deg); }
}

.player-profile-view {
  display: flex;
  flex-direction: column;
  gap: 20px;
  padding: 10px;
  overflow-y: auto;
  height: calc(100vh - 40px);
}

.hextech-profile-card {
  background: linear-gradient(135deg, rgba(30, 35, 40, 0.95), rgba(1, 10, 19, 0.98));
  border: 1px solid #c89b3c;
  padding: 16px;
  border-radius: 6px;
  position: relative;
  box-shadow: 0 4px 20px rgba(0, 0, 0, 0.5), inset 0 0 15px rgba(200, 155, 60, 0.1);
  display: flex;
  flex-direction: column;
  align-items: center;
  text-align: center;
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

.style-badge-frame {
  display: flex;
  align-items: center;
  gap: 8px;
  background: rgba(200, 155, 60, 0.1);
  border: 1px solid #c89b3c;
  padding: 4px 16px;
  border-radius: 20px;
  margin-bottom: 12px;
  box-shadow: 0 0 10px rgba(200, 155, 60, 0.15);
}

.style-sparkle {
  color: #c89b3c;
  font-size: 12px;
  animation: pulse 1.5s infinite;
}

.badge-title {
  font-size: 14px;
  font-weight: 900;
  letter-spacing: 1px;
  color: #f0e6d2;
  text-transform: uppercase;
  text-shadow: 0 0 5px rgba(240, 230, 210, 0.3);
}

.style-description {
  font-size: 11px;
  line-height: 1.5;
  color: #a09b8c;
  max-width: 500px;
}

/* Performance Metrics */
.profile-metrics-card {
  background: rgba(30, 35, 40, 0.4);
  border: 1px solid rgba(255, 255, 255, 0.05);
  border-radius: 4px;
  padding: 16px;
}

.metrics-grid {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 16px;
  margin-top: 12px;
}

.metric-item {
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.metric-label {
  display: flex;
  justify-content: space-between;
  font-size: 10px;
  font-weight: 800;
  text-transform: uppercase;
  color: #a09b8c;
}

.metric-label strong {
  color: #f0e6d2;
  font-size: 11px;
}

.metric-progress-bg {
  height: 6px;
  background: rgba(0, 0, 0, 0.3);
  border-radius: 3px;
  overflow: hidden;
  border: 1px solid rgba(255, 255, 255, 0.03);
}

.metric-progress-bar {
  height: 100%;
  border-radius: 3px;
  transition: width 1s cubic-bezier(0.1, 0.8, 0.1, 1);
}

.metric-progress-bar.gold { background: linear-gradient(90deg, #c89b3c, #f0e6d2); }
.metric-progress-bar.teal { background: linear-gradient(90deg, #008080, #00bff3); }
.metric-progress-bar.blue { background: linear-gradient(90deg, #4c85ff, #8ab4ff); }
.metric-progress-bar.red { background: linear-gradient(90deg, #b30000, #ff4e4e); }

.benchmark {
  font-size: 9px;
  color: rgba(160, 155, 140, 0.6);
  font-weight: 700;
}

.benchmark.text-red {
  color: rgba(255, 78, 78, 0.6);
}

/* AI Coaching Panels */
.coaching-columns {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 16px;
}

.coaching-card {
  background: rgba(30, 35, 40, 0.35);
  border: 1px solid rgba(255, 255, 255, 0.05);
  border-radius: 4px;
  padding: 16px;
}

.coaching-card.mistakes {
  border-left: 3px solid #ff4e4e;
}

.coaching-card.drills {
  border-left: 3px solid #c89b3c;
}

.coaching-card ul {
  list-style: none;
  padding: 0;
  margin: 12px 0 0 0;
  display: flex;
  flex-direction: column;
  gap: 10px;
}

.coaching-card li {
  font-size: 11px;
  line-height: 1.4;
  color: #f0e6d2;
  display: flex;
  align-items: flex-start;
  gap: 8px;
}

.coaching-card .bullet {
  color: #ff4e4e;
  font-weight: 900;
}

.drill-item {
  display: flex;
  align-items: center;
  gap: 10px;
}

.drill-checkbox {
  width: 14px;
  height: 14px;
  border-radius: 50%;
  border: 1px solid #c89b3c;
  color: #c89b3c;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 8px;
  font-weight: 900;
  background: rgba(200, 155, 60, 0.1);
}

.drill-text {
  flex: 1;
}

/* Historic Log List */
.historic-matches-card {
  background: rgba(30, 35, 40, 0.3);
  border: 1px solid rgba(255, 255, 255, 0.04);
  border-radius: 4px;
  padding: 16px;
}

.historic-match-grid {
  display: flex;
  flex-direction: column;
  gap: 8px;
  margin-top: 12px;
}

.historic-match-row {
  display: grid;
  grid-template-columns: 150px 100px 150px 1fr;
  align-items: center;
  padding: 8px 12px;
  background: rgba(0, 0, 0, 0.2);
  border-radius: 4px;
  border: 1px solid rgba(255, 255, 255, 0.02);
}

.historic-match-row.win {
  border-left: 3px solid #4eff9b;
}

.historic-match-row.lose {
  border-left: 3px solid #ff4e4e;
}

.match-champ-info {
  display: flex;
  align-items: center;
  gap: 8px;
}

.match-champ-info img {
  width: 28px;
  height: 28px;
  border-radius: 3px;
  border: 1px solid rgba(255, 255, 255, 0.1);
}

.match-champ-info .details {
  display: flex;
  flex-direction: column;
}

.match-champ-info .champ-name {
  font-size: 11px;
  font-weight: 700;
  color: #f0e6d2;
}

.match-champ-info .position-tag {
  font-size: 8px;
  text-transform: uppercase;
  color: #a09b8c;
}

.match-kda {
  display: flex;
  flex-direction: column;
}

.match-kda .score {
  font-size: 11px;
  font-weight: 800;
  color: #f0e6d2;
}

.match-kda .kda-ratio {
  font-size: 9px;
  color: #a09b8c;
}

.match-performance {
  display: flex;
  flex-direction: column;
  font-size: 10px;
  color: #a09b8c;
}

.match-status-tag {
  text-align: right;
  font-size: 10px;
  font-weight: 900;
  letter-spacing: 0.5px;
}

.historic-match-row.win .match-status-tag { color: #4eff9b; }
.historic-match-row.lose .match-status-tag { color: #ff4e4e; }

/* Placeholder Empty State */
.placeholder-profile {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  text-align: center;
  height: 80%;
  color: #a09b8c;
}

.hextech-crest {
  font-size: 48px;
  margin-bottom: 16px;
  color: #c89b3c;
  animation: pulse 2s infinite;
}

.placeholder-profile h2 {
  font-size: 16px;
  color: #f0e6d2;
  margin-bottom: 8px;
}

.placeholder-profile p {
  font-size: 11px;
  max-width: 320px;
  line-height: 1.5;
}

.placeholder-profile strong {
  color: #c89b3c;
}

/* Champion Mini List & Filter Scoped Styles */
.champion-mini-list {
  display: flex;
  flex-direction: column;
  gap: 4px;
  max-height: 200px;
  overflow-y: auto;
  padding: 4px;
  border-radius: 6px;
  background: rgba(0, 0, 0, 0.2);
  border: 1px solid rgba(200, 155, 60, 0.15);
}

/* Custom scrollbar for mini list */
.champion-mini-list::-webkit-scrollbar {
  width: 4px;
}
.champion-mini-list::-webkit-scrollbar-thumb {
  background: rgba(200, 155, 60, 0.3);
  border-radius: 2px;
}

.champ-mini-btn {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 6px 10px;
  border: 1px solid transparent;
  background: transparent;
  color: #f0e6d2;
  font-size: 11px;
  border-radius: 4px;
  cursor: pointer;
  text-align: left;
  transition: all 0.2s ease;
}

.champ-mini-btn:hover {
  background: rgba(200, 155, 60, 0.15);
  border-color: rgba(200, 155, 60, 0.3);
}

.champ-mini-btn.active {
  background: rgba(200, 155, 60, 0.35);
  border-color: #c89b3c;
  color: #ffffff;
  font-weight: 700;
  box-shadow: 0 0 8px rgba(200, 155, 60, 0.2);
}

.champ-avatar-micro {
  width: 18px;
  height: 18px;
  border-radius: 50%;
  border: 1px solid #c89b3c;
}

.select-field-gold {
  width: 100%;
  padding: 8px 12px;
  background: rgba(10, 15, 25, 0.8);
  border: 1px solid #c89b3c;
  color: #f0e6d2;
  border-radius: 4px;
  font-size: 12px;
  outline: none;
  cursor: pointer;
  transition: border-color 0.2s ease;
}

.select-field-gold:focus {
  border-color: #f0e6d2;
  box-shadow: 0 0 8px rgba(200, 155, 60, 0.3);
}

.input-filter {
  width: 100%;
  padding: 8px 12px;
  background: rgba(0, 0, 0, 0.3);
  border: 1px solid rgba(200, 155, 60, 0.3);
  color: #f0e6d2;
  border-radius: 4px;
  font-size: 11px;
  outline: none;
  transition: all 0.2s ease;
}

.input-filter:focus {
  border-color: #c89b3c;
  background: rgba(0, 0, 0, 0.4);
}

/* Runes Summary Section Scoped Styles */
.runes-summary-section {
  background: rgba(3, 8, 16, 0.45);
  border: 1px solid rgba(200, 155, 60, 0.15);
  border-radius: 8px;
  padding: 16px;
  margin-top: 20px;
  box-shadow: inset 0 0 12px rgba(0, 0, 0, 0.6);
}

.runes-container-grid {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 16px;
  margin-top: 12px;
}

.rune-tree-box {
  background: rgba(1, 4, 8, 0.5);
  border: 1px solid rgba(255, 255, 255, 0.03);
  border-left: 3px solid var(--tree-accent);
  border-radius: 6px;
  padding: 12px;
  display: flex;
  flex-direction: column;
  gap: 10px;
}

.tree-header-row {
  display: flex;
  align-items: center;
  gap: 8px;
  border-bottom: 1px solid rgba(255, 255, 255, 0.05);
  padding-bottom: 6px;
}

.tree-header-icon {
  width: 20px;
  height: 20px;
  object-fit: contain;
}

.tree-header-icon.sm {
  width: 16px;
  height: 16px;
}

.tree-title {
  font-size: 12px;
  font-weight: 800;
  text-transform: uppercase;
  letter-spacing: 0.5px;
  color: var(--tree-accent);
}

.runes-chips-list {
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.rune-chip {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 6px 10px;
  background: rgba(255, 255, 255, 0.02);
  border: 1px solid rgba(255, 255, 255, 0.04);
  border-radius: 4px;
}

.rune-chip.keystone {
  background: rgba(200, 155, 60, 0.05);
  border-color: rgba(200, 155, 60, 0.2);
}

.rune-icon {
  width: 32px;
  height: 32px;
  object-fit: contain;
}

.rune-icon.sm {
  width: 20px;
  height: 20px;
}

.rune-info-text {
  display: flex;
  flex-direction: column;
  gap: 1px;
}

.rune-type {
  font-size: 8px;
  color: #c89b3c;
  text-transform: uppercase;
  font-weight: 700;
  letter-spacing: 0.5px;
}

.rune-name {
  font-size: 11px;
  color: #f0e6d2;
  font-weight: 600;
}

.shards-summary-box {
  border-top: 1px solid rgba(255, 255, 255, 0.05);
  padding-top: 8px;
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.shards-title {
  font-size: 9px;
  font-weight: 700;
  color: #a09b8c;
  text-transform: uppercase;
  letter-spacing: 0.5px;
}

.shards-row {
  display: flex;
  gap: 8px;
}

.shard-chip {
  width: 20px;
  height: 20px;
  border-radius: 50%;
  background: rgba(0, 0, 0, 0.5);
  border: 1px solid rgba(255, 255, 255, 0.1);
  display: flex;
  align-items: center;
  justify-content: center;
}

.shard-icon {
  width: 70%;
  height: 70%;
  object-fit: contain;
}

.btn-reset {
  width: 100%;
  padding: 6px 10px;
  background: rgba(220, 60, 60, 0.12);
  border: 1px solid rgba(220, 60, 60, 0.4);
  color: #ff6b6b;
  border-radius: 4px;
  font-size: 10px;
  font-weight: 700;
  cursor: pointer;
  transition: all 0.2s ease;
  margin-top: 4px;
}

.btn-reset:hover:not(:disabled) {
  background: rgba(220, 60, 60, 0.25);
  border-color: #ff4e4e;
}

.btn-reset:disabled,
.btn-sync:disabled {
  opacity: 0.45;
  cursor: not-allowed;
}

.btn-sync.rune-btn {
  background: rgba(73, 128, 247, 0.12);
  border-color: rgba(73, 128, 247, 0.4);
  color: #7eb3ff;
  margin-top: 4px;
}

.btn-sync.rune-btn:hover:not(:disabled) {
  background: rgba(73, 128, 247, 0.25);
  border-color: #4980f7;
}

.sync-status-msg {
  display: block;
  font-size: 10px;
  color: #a0d4a0;
  line-height: 1.3;
  padding: 4px 0;
  word-break: break-word;
}
</style>
