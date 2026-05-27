<script setup lang="ts">
import { ref, onMounted, computed } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import MatchupExplorerView from "./MatchupExplorerView.vue";
import PlayerProfileView from "./PlayerProfileView.vue";
import { normalizeRuneId, SHARDS_ROWS, TREE_STRUCTURES } from './runesData';

const appWindow = getCurrentWindow();
const isClosing = ref(false);
const minimize = () => appWindow.minimize();
const close = () => { isClosing.value = true; setTimeout(() => appWindow.close(), 300); };

const rawData = ref<any>(null);
const loading = ref(false);
const error = ref<string | null>(null);

const fetchData = async (command: string, args: any = {}) => {
  loading.value = true;
  error.value = null;
  try {
    const res = await invoke(command, args) as any;
    rawData.value = res;
    if (command === 'get_account_by_riot_id' && res.puuid) puuid.value = res.puuid;
  } catch (e: any) {
    error.value = e.toString();
    rawData.value = null;
  } finally {
    loading.value = false;
  }
};

const fetchFullAccountData = async () => {
  if (!gameName.value || !tagLine.value) { error.value = "Preencha o Game Name e a Tag (ex: BR1)"; return; }
  loading.value = true;
  error.value = null;
  try {
    const account: any = await invoke('get_account_by_riot_id', { region: region.value, gameName: gameName.value, tagLine: tagLine.value });
    puuid.value = account.puuid;
    const [summoner, history] = await Promise.all([
      invoke('get_summoner_by_puuid', { platform: platform.value, puuid: account.puuid }),
      invoke('get_match_history', { region: region.value, puuid: account.puuid, count: 5 })
    ]);
    rawData.value = { account, summoner, history };
  } catch (e: any) {
    error.value = e.toString();
    rawData.value = null;
  } finally {
    loading.value = false;
  }
};

const region = ref("americas");
const platform = ref("br1");
const gameName = ref("");
const tagLine = ref("");
const puuid = ref("");
const ddVersion = ref("");

const fetchLatestVersion = async () => {
  try {
    const versions: any = await invoke('get_ddragon_versions');
    if (versions?.length > 0) ddVersion.value = versions[0];
  } catch (e: any) { error.value = "Falha ao buscar versões: " + e.toString(); }
};

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
    if (list?.length > 0 && !searchChamp.value) { searchChamp.value = list[0].id; await fetchChampionData(); }
  } catch (e) { console.error("Erro ao buscar campeões do banco:", e); }
};

const selectChampion = async (id: string) => { searchChamp.value = id; await fetchChampionData(); };

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
  return {
    id: normalId,
    name: `Runa ${id}`,
    icon: `https://raw.communitydragon.org/latest/plugins/rcp-be-lol-game-data/global/default/v1/perk-images/styles/domination/eyeballcollection/eyeballcollection.png`,
    treeId: 0, isKeystone: false, isFallback: true
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
  try { matchupCount.value = await invoke('get_matchup_count_command') as number; }
  catch (e) { console.error(e); }
};

const forceMatchupSync = async () => {
  loading.value = true; error.value = null;
  try { await invoke('sync_matchups_command'); await checkMatchupCount(); }
  catch (e: any) { error.value = e.toString(); }
  finally { loading.value = false; }
};

const syncStatusMsg = ref('');

const resetDb = async () => {
  if (!confirm('⚠️ RESET TOTAL: Todos os dados serão apagados e a aplicação fará uma resincronização completa. Continuar?')) return;
  loading.value = true; error.value = null; syncStatusMsg.value = 'Apagando banco de dados...';
  try {
    const msg = await invoke('reset_builds_and_runes_command') as string;
    syncStatusMsg.value = msg; matchupCount.value = 0; dbChampions.value = []; coreBuild.value = []; championRunes.value = null; rawData.value = null;
  } catch (e: any) { error.value = e.toString(); }
  finally { loading.value = false; }
};

const syncRunes = async () => {
  loading.value = true; error.value = null; syncStatusMsg.value = 'Sincronizando runas do DDragon...';
  try {
    const res = await invoke('sync_and_validate_runes_command') as any;
    syncStatusMsg.value = `✅ Runas OK: ${res.fixed} de ${res.total_checked} corrigidos (v${res.version})`;
    await fetchDbChampions();
  } catch (e: any) { error.value = e.toString(); syncStatusMsg.value = ''; }
  finally { loading.value = false; }
};

const waitForDb = async () => {
  try {
    const ready = await invoke('is_db_ready');
    if (ready) { isDbReady.value = true; await checkMatchupCount(); await fetchLatestVersion(); await fetchDbChampions(); }
    else setTimeout(waitForDb, 200);
  } catch (e) { console.error("Erro ao checar inicialização do banco:", e); setTimeout(waitForDb, 200); }
};

onMounted(() => { waitForDb(); fetchLcuSummoner(); });

const activeTab = ref<'matchup' | 'profile'>('matchup');
const riotPuuid = ref("");
const styleProfile = ref<any>(null);
const analyzedCoaching = ref<any>(null);
const isAnalyzingProfile = ref(false);

const fetchLcuSummoner = async () => {
  try {
    const summoner: any = await invoke("get_current_summoner");
    if (summoner?.puuid) {
      riotPuuid.value = summoner.puuid;
      gameName.value = summoner.gameName || summoner.displayName || "";
      tagLine.value = summoner.tagLine || "";
      await invoke("save_lcu_summoner_command", { puuid: summoner.puuid, gameName: gameName.value, tagLine: tagLine.value });
    } else { await loadSavedSummoner(); }
  } catch (e) { console.warn("LCU not connected:", e); await loadSavedSummoner(); }
};

const loadSavedSummoner = async () => {
  try {
    const saved: any = await invoke("get_saved_summoner_command");
    if (saved) { riotPuuid.value = saved.puuid || ""; gameName.value = saved.game_name || ""; tagLine.value = saved.tag_line || ""; }
  } catch (err) { console.error("Erro ao ler summoner:", err); }
};

const fetchPlayerStyleAnalysis = async () => {
  if (!riotPuuid.value) { error.value = "Insira um PUUID válido ou conecte o cliente do LoL."; return; }
  isAnalyzingProfile.value = true; error.value = null; styleProfile.value = null; analyzedCoaching.value = null;
  try {
    const profile = await invoke("get_player_style_analysis", { puuid: riotPuuid.value, region: region.value, count: 15 }) as any;
    styleProfile.value = profile;
    if (profile.coaching_tips) {
      try { analyzedCoaching.value = JSON.parse(profile.coaching_tips); }
      catch (err) { console.error("Erro ao fazer parse dos treinos da IA:", err); }
    }
    try { await invoke("save_lcu_summoner_command", { puuid: riotPuuid.value, gameName: gameName.value || "", tagLine: tagLine.value || "" }); }
    catch (dbErr) { console.warn("Falha ao salvar summoner:", dbErr); }
  } catch (e: any) { error.value = e.toString(); }
  finally { isAnalyzingProfile.value = false; }
};

const fetchChampionData = async () => {
  if (!searchChamp.value) return;
  loading.value = true; error.value = null; matchups.value = []; coreBuild.value = []; situationalItems.value = []; championRunes.value = null;
  try {
    const matchRes = await invoke('get_champion_matchups_command', { champId: searchChamp.value }) as any;
    resolvedChampId.value = matchRes.champion_id;
    const list = matchRes.matchups;
    matchups.value = list;
    bestMatchups.value = list.length > 0 ? [...list].sort((a: any, b: any) => b.win_rate - a.win_rate).slice(0, 5) : [];
    worstMatchups.value = list.length > 0 ? [...list].sort((a: any, b: any) => a.win_rate - b.win_rate).slice(0, 5) : [];
    const buildRes = await invoke('get_champion_build_command', { champId: resolvedChampId.value }) as any[];
    if (buildRes.length > 0) coreBuild.value = buildRes[0];
    try {
      const runesRes = await invoke('get_champion_runes_command', { champId: resolvedChampId.value }) as any;
      championRunes.value = runesRes;
    } catch (err) { console.warn("Nenhuma runa recomendada:", err); championRunes.value = null; }
    const sitRes = await invoke('get_situational_items_command', { champId: resolvedChampId.value }) as any[];
    situationalItems.value = sitRes;
    try {
      const tipsRes = await invoke('get_tactical_tips_command', { champId: resolvedChampId.value }) as any;
      tacticalTips.value = tipsRes;
    } catch (err) { console.error("Erro ao carregar dicas táticas:", err); }
    rawData.value = { matchups: matchRes, builds: buildRes, situational: sitRes, runes: championRunes.value };
  } catch (e: any) { error.value = e.toString(); }
  finally { loading.value = false; }
};
</script>

<template>
  <div class="flex w-screen h-screen bg-[#010a13] text-[#f0e6d2] transition-all duration-300 animate-[slideUp_0.4s_ease-out_forwards]"
       :class="{ 'opacity-0 scale-95': isClosing }">

    <!-- Sidebar -->
    <aside class="w-70 shrink-0 bg-[rgba(30,35,40,0.5)] border-r border-[#c89b3c] p-4 flex flex-col gap-2 overflow-y-auto">

      <header data-tauri-drag-region class="flex justify-between items-center mb-5 border-b border-white/10 pb-2.5 cursor-move shrink-0">
        <h3 data-tauri-drag-region class="text-[12px] text-[#c89b3c] uppercase m-0 font-bold tracking-wide">Data Explorer</h3>
        <div class="flex gap-1">
          <button class="bg-transparent border-none text-[#c89b3c] text-sm cursor-pointer px-1.5 py-0.5 rounded transition-all hover:bg-white/10" @click="minimize">_</button>
          <button class="bg-transparent border-none text-[#c89b3c] text-sm cursor-pointer px-1.5 py-0.5 rounded transition-all hover:bg-[#ff4e4e] hover:text-white" @click="close">×</button>
        </div>
      </header>

      <!-- Tab Switcher -->
      <div class="grid grid-cols-2 gap-2 mb-4 bg-black/20 p-1 rounded border border-[rgba(200,155,60,0.2)] shrink-0">
        <button
          :class="['py-1.5 text-center text-[11px] font-extrabold cursor-pointer rounded-sm transition-all border-none',
            activeTab === 'matchup' ? 'bg-[#c89b3c] text-[#010a13] shadow-[0_0_8px_rgba(200,155,60,0.4)]' : 'bg-transparent text-[#a09b8c] hover:bg-white/5 hover:text-[#f0e6d2]']"
          @click="activeTab = 'matchup'">⚔️ Confrontos</button>
        <button
          :class="['py-1.5 text-center text-[11px] font-extrabold cursor-pointer rounded-sm transition-all border-none',
            activeTab === 'profile' ? 'bg-[#c89b3c] text-[#010a13] shadow-[0_0_8px_rgba(200,155,60,0.4)]' : 'bg-transparent text-[#a09b8c] hover:bg-white/5 hover:text-[#f0e6d2]']"
          @click="activeTab = 'profile'">👤 Estilo & IA</button>
      </div>

      <!-- Tab 1 Sidebar: Champion Search -->
      <div v-if="activeTab === 'matchup'" class="flex flex-col gap-3 animate-[fadeIn_0.3s_ease-out_forwards]">
        <section class="flex flex-col">
          <h3 class="text-[12px] text-[#c89b3c] uppercase mb-2 font-bold">Explorar Campeão</h3>
          <div class="flex flex-col gap-1">
            <input v-model="searchChamp" placeholder="Buscar campeão (ex: Jinx...)"
              class="bg-[#1e2328] border border-white/10 text-white px-1.5 py-1.5 text-[11px] outline-none focus:border-[#c89b3c] transition-colors"
              @keyup.enter="fetchChampionData" />
            <button class="bg-linear-to-b from-[#c89b3c] to-[#785a28] text-[#1e2328] border border-[#f0e6d2] font-bold py-2 cursor-pointer uppercase text-[10px] transition-all hover:opacity-90"
                    @click="fetchChampionData">Analisar</button>
          </div>
          <div v-if="filteredChampions.length > 0"
               class="champion-mini-list flex flex-col gap-1 max-h-50 overflow-y-auto p-1 rounded-md bg-black/20 border border-[rgba(200,155,60,0.15)] mt-3">
            <button
              v-for="c in filteredChampions.slice(0, 15)"
              :key="c.id"
              :class="['flex items-center gap-2 px-2.5 py-1.5 border rounded text-[#f0e6d2] text-[11px] cursor-pointer text-left transition-all',
                searchChamp && searchChamp.toLowerCase() === c.id.toLowerCase()
                  ? 'bg-[rgba(200,155,60,0.35)] border-[#c89b3c] text-white font-bold shadow-[0_0_8px_rgba(200,155,60,0.2)]'
                  : 'bg-transparent border-transparent hover:bg-[rgba(200,155,60,0.15)] hover:border-[rgba(200,155,60,0.3)]']"
              @click="selectChampion(c.id)">
              <img :src="`https://ddragon.leagueoflegends.com/cdn/${ddVersion || '16.10.1'}/img/champion/${c.id}.png`"
                   class="w-4.5 h-4.5 rounded-full border border-[#c89b3c]" />
              <span>{{ c.name }}</span>
            </button>
          </div>
        </section>

        <section class="flex flex-col mt-6">
          <h3 class="text-[12px] text-[#c89b3c] uppercase mb-2 font-bold">Banco de Dados</h3>
          <div class="bg-black/30 border border-white/5 rounded p-2.5 flex flex-col gap-2">
            <span class="text-[10px] text-[#a09b8c] text-center">{{ matchupCount }} Registros</span>
            <span v-if="syncStatusMsg" class="block text-[10px] text-[#a0d4a0] leading-snug py-1 wrap-break-word">{{ syncStatusMsg }}</span>
            <button class="bg-[#4ab4f0] text-white border-none py-1.5 rounded-sm font-extrabold text-[9px] uppercase text-center cursor-pointer transition-all hover:opacity-90 disabled:opacity-45 disabled:cursor-not-allowed"
                    @click="forceMatchupSync" :disabled="loading">
              <span v-if="loading">Sincronizando...</span>
              <span v-else>Forçar Sincronização</span>
            </button>
            <button class="mt-1 bg-[rgba(73,128,247,0.12)] border border-[rgba(73,128,247,0.4)] text-[#7eb3ff] py-1.5 px-2.5 rounded text-[10px] font-bold cursor-pointer transition-all hover:bg-[rgba(73,128,247,0.25)] hover:border-[#4980f7] disabled:opacity-45 disabled:cursor-not-allowed"
                    @click="syncRunes" :disabled="loading" title="Sincroniza runas do DDragon e valida 4+2+3">
              🔮 Validar Runas (4+2+3)
            </button>
            <button class="w-full px-2.5 py-1.5 bg-[rgba(220,60,60,0.12)] border border-[rgba(220,60,60,0.4)] text-[#ff6b6b] rounded text-[10px] font-bold cursor-pointer transition-all mt-1 hover:bg-[rgba(220,60,60,0.25)] hover:border-[#ff4e4e] disabled:opacity-45 disabled:cursor-not-allowed"
                    @click="resetDb" :disabled="loading" title="Apaga todos os dados e força resincronização completa">
              🗑️ Reset Total do Banco
            </button>
          </div>
        </section>
      </div>

      <!-- Tab 2 Sidebar: Profile Analysis -->
      <div v-else-if="activeTab === 'profile'" class="flex flex-col gap-3 animate-[fadeIn_0.3s_ease-out_forwards]">
        <section class="flex flex-col">
          <h3 class="text-[12px] text-[#c89b3c] uppercase mb-2 font-bold">Análise de Estilo</h3>
          <div class="flex flex-col gap-1">
            <label class="text-[9px] text-[#c89b3c] uppercase font-extrabold tracking-[0.5px] mt-1.5">Riot PUUID</label>
            <input v-model="riotPuuid" placeholder="Cole seu PUUID ou conecte o LoL"
              class="bg-[#1e2328] border border-white/10 text-white px-1.5 py-1.5 text-[11px] outline-none focus:border-[#c89b3c] transition-colors" />
            <label class="text-[9px] text-[#c89b3c] uppercase font-extrabold tracking-[0.5px] mt-1.5">Região (Partidas V5)</label>
            <select v-model="region"
              class="bg-[#1e2328] border border-white/10 text-white px-1.5 py-1.5 text-[11px] cursor-pointer outline-none w-full focus:border-[#c89b3c]">
              <option value="americas">Americas (BR, NA, LA)</option>
              <option value="europe">Europe</option>
              <option value="asia">Asia</option>
            </select>
            <button class="mt-3 bg-linear-to-b from-[#c89b3c] to-[#785a28] text-[#1e2328] border border-[#f0e6d2] font-bold py-2 cursor-pointer uppercase text-[10px] transition-all hover:opacity-90"
                    @click="fetchPlayerStyleAnalysis">
              <span v-if="isAnalyzingProfile">Analisando...</span>
              <span v-else>💡 Analisar Estilo</span>
            </button>
          </div>
          <div v-if="styleProfile" class="mt-6 bg-black/20 border border-[rgba(200,155,60,0.15)] p-2 rounded text-[10px]">
            <div>Estilo: <strong class="text-[#c89b3c]">{{ styleProfile.style_tag }}</strong></div>
            <div>Histórico: {{ styleProfile.total_games }} jogos analisados</div>
          </div>
        </section>
      </div>

      <!-- Debug Tools -->
      <section class="flex flex-col mt-6">
        <h3 class="text-[12px] text-[#c89b3c] uppercase mb-2 font-bold">Ferramentas de Debug</h3>
        <details class="bg-white/2 rounded overflow-hidden">
          <summary class="text-[10px] text-[#a09b8c] px-2.5 py-1.5 cursor-pointer select-none border-b border-white/5 list-none hover:bg-white/5 hover:text-white">LCU / LCA (Local)</summary>
          <div class="grid grid-cols-2 gap-1 p-2">
            <button class="text-[9px] py-1 bg-white/5 border border-[rgba(200,155,60,0.3)] text-[#f0e6d2] cursor-pointer transition-all hover:bg-[rgba(200,155,60,0.1)] hover:border-[#c89b3c]" @click="fetchData('get_lcu_status')">Status</button>
            <button class="text-[9px] py-1 bg-white/5 border border-[rgba(200,155,60,0.3)] text-[#f0e6d2] cursor-pointer transition-all hover:bg-[rgba(200,155,60,0.1)] hover:border-[#c89b3c]" @click="fetchData('get_current_summoner')">Summoner</button>
            <button class="text-[9px] py-1 bg-white/5 border border-[rgba(200,155,60,0.3)] text-[#f0e6d2] cursor-pointer transition-all hover:bg-[rgba(200,155,60,0.1)] hover:border-[#c89b3c]" @click="fetchData('get_all_game_data')">LCA Data</button>
          </div>
        </details>
        <details class="mt-3 bg-white/2 rounded overflow-hidden">
          <summary class="text-[10px] text-[#a09b8c] px-2.5 py-1.5 cursor-pointer select-none border-b border-white/5 list-none hover:bg-white/5 hover:text-white">Riot Web API</summary>
          <div class="flex flex-col gap-1 p-2">
            <input v-model="gameName" placeholder="Game Name"
              class="bg-[#1e2328] border border-white/10 text-white px-1.5 py-1 text-[11px] outline-none focus:border-[#c89b3c] transition-colors" />
            <input v-model="tagLine" placeholder="Tag (ex: BR1)"
              class="bg-[#1e2328] border border-white/10 text-white px-1.5 py-1 text-[11px] outline-none focus:border-[#c89b3c] transition-colors" />
            <button class="bg-[#4ab4f0] text-white border-none font-bold py-2 text-[12px] cursor-pointer transition-all hover:opacity-90" @click="fetchFullAccountData">Full Sync</button>
          </div>
        </details>
        <details class="mt-3 bg-white/2 rounded overflow-hidden">
          <summary class="text-[10px] text-[#a09b8c] px-2.5 py-1.5 cursor-pointer select-none border-b border-white/5 list-none hover:bg-white/5 hover:text-white">Data Dragon</summary>
          <div class="grid grid-cols-2 gap-1 p-2">
            <button class="text-[9px] py-1 bg-white/5 border border-[rgba(200,155,60,0.3)] text-[#f0e6d2] cursor-pointer transition-all hover:bg-[rgba(200,155,60,0.1)] hover:border-[#c89b3c]" @click="fetchLatestVersion">Update Ver</button>
            <input v-model="ddVersion" placeholder="Version"
              class="col-span-2 bg-[#1e2328] border border-white/10 text-white px-1.5 py-1 text-[9px] outline-none" />
            <button class="text-[9px] py-1 bg-white/5 border border-[rgba(200,155,60,0.3)] text-[#f0e6d2] cursor-pointer transition-all hover:bg-[rgba(200,155,60,0.1)] hover:border-[#c89b3c]" @click="fetchData('get_ddragon_champions')">Champs</button>
          </div>
        </details>
      </section>

      <div class="mt-auto flex justify-between text-[9px] opacity-30 pt-5">
        <span>v{{ ddVersion || '...' }}</span>
        <span>Spell Coach IA</span>
      </div>
    </aside>

    <!-- Main Content -->
    <main class="flex-1 p-5 overflow-auto bg-[#010a13]">
      <div v-if="loading" class="text-[#4ab4f0]">Buscando dados...</div>
      <div v-else-if="error" class="text-[#ff4e4e] bg-[rgba(255,78,78,0.1)] p-3 border border-[#ff4e4e]">Erro: {{ error }}</div>

      <div v-else-if="activeTab === 'matchup'" class="w-full h-full">
        <MatchupExplorerView
          v-if="resolvedChampId"
          :resolved-champ-id="resolvedChampId"
          :best-matchups="bestMatchups"
          :worst-matchups="worstMatchups"
          :core-build="coreBuild"
          :situational-items="situationalItems"
          :champion-runes="championRunes"
          :tactical-tips="tacticalTips"
          :dd-version="ddVersion"
          :find-rune-details="findRuneDetails"
          :find-shard-details="findShardDetails"
        />
        <div v-else class="text-white/30 flex items-center justify-center h-full italic">
          Selecione uma chamada ou busque um campeão para visualizar os dados.
        </div>
      </div>

      <PlayerProfileView
        v-else-if="activeTab === 'profile'"
        :style-profile="styleProfile"
        :analyzed-coaching="analyzedCoaching"
        :is-analyzing-profile="isAnalyzingProfile"
        :dd-version="ddVersion"
      />
    </main>
  </div>
</template>

<style scoped>
@keyframes slideUp {
  from { opacity: 0; transform: translateY(20px); }
  to   { opacity: 1; transform: translateY(0); }
}
@keyframes fadeIn {
  from { opacity: 0; transform: translateY(10px); }
  to   { opacity: 1; transform: translateY(0); }
}
.champion-mini-list::-webkit-scrollbar { width: 4px; }
.champion-mini-list::-webkit-scrollbar-thumb { background: rgba(200, 155, 60, 0.3); border-radius: 2px; }
</style>
