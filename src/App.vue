<script setup lang="ts">
/*
===============================================================================
                     SPELL COACH IA - VIEW PRINCIPAL (App.vue)
===============================================================================
Este é o componente raiz (Root View) do nosso aplicativo Vue 3.

Para um estudante de programação:
* O que mudou aqui?
  Anteriormente, este arquivo continha quase 500 linhas, misturando HTML, CSS e 
  toda a lógica de rede do Tauri. Agora, aplicamos a separação de conceitos!
  Toda a lógica e estados reativos foram movidos para o Composable './composables/useSpellCoach'.
  Aqui no 'App.vue', nós apenas importamos o 'useSpellCoach' e destruturamos os 
  dados de que a tela precisa. O arquivo ficou extremamente limpo e legível!
===============================================================================
*/

// 1. IMPORTAÇÃO DOS COMPONENTES VISUAIS (Apresentação)
import BuildWindow from "./components/BuildWindow.vue";
import SettingsWindow from "./components/SettingsWindow.vue";
import DataViewerWindow from "./components/DataViewerWindow.vue";
import Flashcard from "./components/Flashcard.vue";
import SyncProgress from "./components/SyncProgress.vue";
import RuneOverlay from "./components/RuneOverlay.vue";
import WardMapCard from "./components/WardMapCard.vue";
import PostGameOverlay from "./components/PostGameOverlay.vue";

// 2. IMPORTAÇÃO DA LÓGICA DE NEGÓCIO MODULARIZADA (Composable)
import { useSpellCoach } from "./composables/useSpellCoach";

// 3. DESTRUTURAÇÃO DO ESTADO E MÉTODOS
// Nós chamamos a função 'useSpellCoach()' e extraímos apenas as variáveis e ações
// que serão lidas ou disparadas no template HTML logo abaixo.
const {
  appWindow,
  windowLabel,
  settingsLoading,
  dataViewerLoading,
  summonerName,
  lcuStatus,
  gameFlowState,
  flashcardKey,
  isExitingFlashcard,
  flashcardData,
  wardMapData,
  postGameReport,
  postGameLoading,
  openSettings,
  openDataViewer,
  toggleFlashcard,
  showPostGame,
} = useSpellCoach();
</script>

<template>
  <!-- 
    ===========================================================================
    TEMPLATE MULTI-JANELA DINÂMICO
    ===========================================================================
    Para o estudante:
    Como o Tauri utiliza o mesmo arquivo HTML de base, usamos condicionais Vue 'v-if' 
    e 'v-else-if' lendo a propriedade 'windowLabel' para decidir qual componente visual 
    deve ser renderizado em cada janela webview aberta!
  -->

  <!-- Janela Principal (Main Bar overlay no topo da tela do jogador) -->
  <!-- @mouseenter/@mouseleave togglam setIgnoreCursorEvents para que cliques no jogo
       passem através da barra quando o cursor não estiver sobre ela -->
  <div v-if="windowLabel === 'main'" class="app-container" :class="{ 'expanded': gameFlowState !== 'IDLE' }"
       @mouseenter="appWindow.setIgnoreCursorEvents(false)"
       @mouseleave="appWindow.setIgnoreCursorEvents(true)">
    <div class="overlay-header glass" data-tauri-drag-region>
      <div class="header-left" data-tauri-drag-region>
        <div class="ia-badge ready">⚡ COACH</div>
        <div class="drag-dots"><span></span><span></span><span></span></div>
      </div>
      <div class="header-center" data-tauri-drag-region>
        <span class="summoner-text">{{ summonerName }}</span>
        <span class="state-text">• {{ gameFlowState }}</span>
      </div>
      <div class="header-right">
        <span class="lcu-status" :class="lcuStatus.toLowerCase()">{{ lcuStatus }}</span>
        <div class="action-btns">
          <button @click="showPostGame" title="Análise da Última Partida" :class="{ 'loading': postGameLoading }">📊</button>
          <button @click="openDataViewer" title="Perfil & Estatísticas IA" :class="{ 'loading': dataViewerLoading }">👤</button>
          <button @click="openSettings" title="Configurações" :class="{ 'loading': settingsLoading }">⚙️</button>
          <button @click="appWindow.minimize()">_</button>
          <button @click="appWindow.close()" class="close-btn">×</button>
        </div>
      </div>
    </div>
  </div>

  <!-- Janela de Configurações -->
  <SettingsWindow v-else-if="windowLabel === 'settings'" />

  <!-- Janela da Barra de Builds dentro do jogo ativo -->
  <div v-else-if="windowLabel === 'build'" class="build-window-container"><BuildWindow /></div>

  <!-- Janela do Widget de Dicas Rápidas (Flashcard) com animações dinâmicas de slide -->
  <div v-else-if="windowLabel === 'flashcard'" :key="flashcardKey" class="strip-window-wrapper" :class="isExitingFlashcard ? 'strip-exit' : 'strip-entry'">
    <div class="strip-header"><div class="strip-badge">DICA RÁPIDA</div><div class="strip-line"></div></div>
    <Flashcard :title="flashcardData.title" :frontText="flashcardData.frontText" :backText="flashcardData.backText" :rarity="flashcardData.rarity" />
  </div>

  <!-- Janela do Overlay flutuante de Runas Recomendadas -->
  <div v-else-if="windowLabel === 'rune-overlay'" class="rune-overlay-wrapper"><RuneOverlay /></div>

  <!-- Janela do Visualizador de Perfis e Gráficos IA -->
  <DataViewerWindow v-else-if="windowLabel === 'data-viewer'" />

  <!-- Janela do Progresso de Sincronização -->
  <div v-else-if="windowLabel === 'sync-progress'" class="sync-progress-window"><SyncProgress /></div>

  <!-- Janela do Ward Map (minimap com pontos de sentinela) -->
  <div v-else-if="windowLabel === 'ward-map'" class="ward-map-window">
    <WardMapCard
      :champion="wardMapData.champion"
      :role="wardMapData.role"
      :phase="wardMapData.phase"
      :team-side="wardMapData.teamSide"
      :wards="wardMapData.wards"
      :game-time="wardMapData.gameTime"
      :objective="wardMapData.objective"
      :objective-emoji="wardMapData.objectiveEmoji"
      :seconds-to-spawn="wardMapData.secondsToSpawn"
    />
  </div>

  <!-- Janela de Análise Pós-Jogo -->
  <div v-else-if="windowLabel === 'post-game'" class="postgame-window">
    <PostGameOverlay
      v-if="postGameReport"
      :report="postGameReport"
      @close="appWindow.close()"
    />
  </div>

  <!-- Fallback de depuração de segurança -->
  <div v-else class="fallback-debug">Window: {{ windowLabel }}</div>
</template>

<style scoped>
/*
===============================================================================
ESTILOS SCOPED (Vanila CSS)
===============================================================================
Estilos estruturais e visuais da nossa barra principal e contêineres de widgets.
Usamos layouts Flexbox e CSS Grid para manter tudo 100% responsivo e otimizado.
*/
.app-container { width: 400px; height: 50px; display: flex; flex-direction: column; overflow: hidden; padding: 5px; background: transparent !important; }
.build-window-container { width: 480px; height: 60px; display: flex; align-items: center; justify-content: center; padding: 3px; background: transparent !important; overflow: hidden; }
.strip-window-wrapper { width: 100%; height: 100%; display: flex; flex-direction: column; align-items: flex-end; justify-content: center; padding-right: 10px; background: transparent !important; overflow: hidden; }
.strip-header { width: 300px; display: flex; align-items: center; gap: 8px; margin-bottom: 6px; padding: 0 4px; }
.strip-badge { font-size: 9px; font-weight: 800; color: #f0e6d2; background: #c89b3c; padding: 2px 6px; border-radius: 2px; letter-spacing: 1px; }
.strip-line { flex: 1; height: 1px; background: linear-gradient(90deg, #c89b3c, transparent); opacity: 0.3; }
.strip-entry { animation: stripSlideIn 0.6s cubic-bezier(0.23, 1, 0.32, 1) forwards; will-change: transform, opacity; }
.strip-exit { animation: stripSlideOut 0.5s cubic-bezier(0.23, 1, 0.32, 1) forwards; will-change: transform, opacity; }
@keyframes stripSlideIn { from { transform: translateX(350px) translateZ(0); opacity: 0; } to { transform: translateX(0) translateZ(0); opacity: 1; } }
@keyframes stripSlideOut { from { transform: translateX(0) translateZ(0); opacity: 1; } to { transform: translateX(350px) translateZ(0); opacity: 0; } }
.overlay-header { height: 40px; display: grid; grid-template-columns: 70px 1fr auto; align-items: center; padding: 0 8px; background: rgba(1, 10, 19, 0.98); border: 1px solid #c8aa6e; border-radius: 4px; z-index: 10; }
.header-left { display: flex; align-items: center; gap: 6px; }
.ia-badge { font-size: 8px; font-weight: 800; padding: 1px 5px; border-radius: 3px; background: rgba(200, 170, 110, 0.15); border: 1px solid #c8aa6e; color: #c8aa6e; letter-spacing: 0.5px; }
.ia-badge.ready { border-color: #c8aa6e; color: #c8aa6e; }
.drag-dots { display: flex; flex-direction: column; gap: 2px; opacity: 0.4; }
.drag-dots span { width: 2px; height: 2px; background: white; border-radius: 50%; }
.header-center { display: flex; align-items: center; justify-content: center; gap: 4px; overflow: hidden; white-space: nowrap; }
.summoner-text { font-size: 10px; font-weight: 800; color: #c8aa6e; overflow: hidden; text-overflow: ellipsis; }
.state-text { font-size: 8px; font-weight: 700; color: #a09b8c; text-transform: uppercase; }
.header-right { display: flex; align-items: center; justify-content: flex-end; gap: 6px; }
.lcu-status { font-size: 7px; font-weight: 800; color: #ff4e4e; text-transform: uppercase; }
.lcu-status.connected { color: #4eff9b; }
.action-btns { display: flex; gap: 2px; border-left: 1px solid rgba(255,255,255,0.1); padding-left: 6px; }
.action-btns button { background: none; border: none; color: white; opacity: 0.5; cursor: pointer; font-size: 12px; padding: 2px 4px; }
.action-btns button:hover { opacity: 1; }
.action-btns button.loading { opacity: 1; color: #c8aa6e; animation: pulse 1s infinite; }
.action-btns button.close-btn:hover { color: #ff4e4e; }
@keyframes pulse { 0% { transform: scale(1); opacity: 0.5; } 50% { transform: scale(1.2); opacity: 1; } 100% { transform: scale(1); opacity: 0.5; } }
.sync-progress-window { width: 400px; height: 160px; display: flex; align-items: center; justify-content: center; background: transparent !important; }
.rune-overlay-wrapper { width: 100%; height: 100%; display: flex; align-items: center; justify-content: center; background: transparent !important; overflow: hidden; }
.ward-map-window { width: 100%; height: 100%; display: flex; align-items: center; justify-content: center; background: transparent !important; }
.postgame-window { width: 100%; height: 100%; display: flex; align-items: center; justify-content: center; background: transparent !important; }
.fallback-debug { padding: 20px; background: rgba(0,0,0,0.8); color: white; border: 1px solid red; }
</style>