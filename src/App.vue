<script setup lang="ts">
import BuildWindow from "./components/BuildWindow.vue";
import SettingsWindow from "./components/SettingsWindow.vue";
import DataViewerWindow from "./components/DataViewerWindow.vue";
import Flashcard from "./components/Flashcard.vue";
import SyncProgress from "./components/SyncProgress.vue";
import RuneOverlay from "./components/RuneOverlay.vue";
import WardMapCard from "./components/WardMapCard.vue";
import PostGameOverlay from "./components/PostGameOverlay.vue";
import PreGameOverlay from "./components/PreGameOverlay.vue";
import UpdaterWindow from "./components/UpdaterWindow.vue";
import { useSpellCoach } from "./composables/useSpellCoach";

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
  showPostGame,
} = useSpellCoach();
</script>

<template>
  <!-- Main bar overlay -->
  <div v-if="windowLabel === 'main'"
       class="w-100 h-12.5 flex flex-col overflow-hidden p-1.25 bg-transparent!"
       :class="{ 'expanded': gameFlowState !== 'IDLE' }">
    <div class="h-10 grid grid-cols-[70px_1fr_auto] items-center px-2 bg-[rgba(1,10,19,0.98)] border border-[#c8aa6e] rounded z-10">
      <div class="flex items-center gap-1.5" data-tauri-drag-region>
        <div class="text-[8px] font-extrabold py-px px-1.25 rounded-sm bg-[rgba(200,170,110,0.15)] border border-[#c8aa6e] text-[#c8aa6e] tracking-[0.5px]">⚡ COACH</div>
        <div class="flex flex-col gap-0.5 opacity-40" data-tauri-drag-region>
          <span class="w-0.5 h-0.5 bg-white rounded-full"></span>
          <span class="w-0.5 h-0.5 bg-white rounded-full"></span>
          <span class="w-0.5 h-0.5 bg-white rounded-full"></span>
        </div>
      </div>
      <div class="flex items-center justify-center gap-1 overflow-hidden whitespace-nowrap" data-tauri-drag-region>
        <span class="text-[10px] font-extrabold text-[#c8aa6e] overflow-hidden text-ellipsis">{{ summonerName }}</span>
        <span class="text-[8px] font-bold text-[#a09b8c] uppercase">• {{ gameFlowState }}</span>
      </div>
      <div class="flex items-center justify-end gap-1.5">
        <span class="text-[7px] font-extrabold uppercase"
              :class="lcuStatus.toLowerCase() === 'connected' ? 'text-[#4eff9b]' : 'text-[#ff4e4e]'">{{ lcuStatus }}</span>
        <div class="flex gap-0.5 border-l border-white/10 pl-1.5">
          <button class="bg-transparent border-none text-white opacity-50 cursor-pointer text-[12px] px-1 py-0.5 hover:opacity-100 transition-opacity"
                  :class="{ 'loading': postGameLoading }"
                  @click="showPostGame" title="Análise da Última Partida">📊</button>
          <button class="bg-transparent border-none text-white opacity-50 cursor-pointer text-[12px] px-1 py-0.5 hover:opacity-100 transition-opacity"
                  :class="{ 'loading': dataViewerLoading }"
                  @click="openDataViewer" title="Perfil & Estatísticas IA">👤</button>
          <button class="bg-transparent border-none text-white opacity-50 cursor-pointer text-[12px] px-1 py-0.5 hover:opacity-100 transition-opacity"
                  :class="{ 'loading': settingsLoading }"
                  @click="openSettings" title="Configurações">⚙️</button>
          <button class="bg-transparent border-none text-white opacity-50 cursor-pointer text-[12px] px-1 py-0.5 hover:opacity-100 transition-opacity"
                  @click="appWindow.minimize()">_</button>
          <button class="bg-transparent border-none text-white opacity-50 cursor-pointer text-[12px] px-1 py-0.5 hover:opacity-100 hover:text-[#ff4e4e] transition-all"
                  @click="appWindow.close()">×</button>
        </div>
      </div>
    </div>
  </div>

  <SettingsWindow v-else-if="windowLabel === 'settings'" />

  <div v-else-if="windowLabel === 'build'" class="w-120 h-15 flex items-center justify-center p-0.75 bg-transparent! overflow-hidden">
    <BuildWindow />
  </div>

  <div v-else-if="windowLabel === 'flashcard'"
       :key="flashcardKey"
       class="w-full h-full flex flex-col items-end justify-center pr-2.5 bg-transparent! overflow-hidden"
       :class="isExitingFlashcard ? 'strip-exit' : 'strip-entry'">
    <div class="w-75 flex items-center gap-2 mb-1.5 px-1">
      <div class="text-[9px] font-extrabold text-[#f0e6d2] bg-[#c89b3c] px-1.5 py-0.5 rounded-sm tracking-[1px]">DICA RÁPIDA</div>
      <div class="flex-1 h-px bg-linear-to-r from-[#c89b3c] to-transparent opacity-30"></div>
    </div>
    <Flashcard :title="flashcardData.title" :frontText="flashcardData.frontText" :backText="flashcardData.backText" :rarity="flashcardData.rarity" />
  </div>

  <div v-else-if="windowLabel === 'rune-overlay'" class="w-full h-full flex items-center justify-center bg-transparent! overflow-hidden">
    <RuneOverlay />
  </div>

  <DataViewerWindow v-else-if="windowLabel === 'data-viewer'" />

  <div v-else-if="windowLabel === 'sync-progress'" class="w-100 h-40 flex items-center justify-center bg-transparent!">
    <SyncProgress />
  </div>

  <div v-else-if="windowLabel === 'ward-map'" class="w-full h-full flex items-center justify-center bg-transparent!">
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

  <div v-else-if="windowLabel === 'post-game'" class="w-full h-full flex items-center justify-center bg-transparent!">
    <PostGameOverlay v-if="postGameReport" :report="postGameReport" @close="appWindow.close()" />
  </div>

  <div v-else-if="windowLabel === 'pre-game'" class="w-full h-full flex items-start justify-start bg-transparent!">
    <PreGameOverlay />
  </div>

  <div v-else-if="windowLabel === 'updater'" class="w-full h-full">
    <UpdaterWindow />
  </div>

  <div v-else class="p-5 bg-black/80 text-white border border-red-500">Window: {{ windowLabel }}</div>
</template>

<style scoped>
@keyframes stripSlideIn {
  from { transform: translateX(350px) translateZ(0); opacity: 0; }
  to   { transform: translateX(0) translateZ(0); opacity: 1; }
}
@keyframes stripSlideOut {
  from { transform: translateX(0) translateZ(0); opacity: 1; }
  to   { transform: translateX(350px) translateZ(0); opacity: 0; }
}
@keyframes pulse-icon {
  0%   { transform: scale(1); opacity: 0.5; }
  50%  { transform: scale(1.2); opacity: 1; }
  100% { transform: scale(1); opacity: 0.5; }
}
.strip-entry { animation: stripSlideIn 0.6s cubic-bezier(0.23, 1, 0.32, 1) forwards; will-change: transform, opacity; }
.strip-exit  { animation: stripSlideOut 0.5s cubic-bezier(0.23, 1, 0.32, 1) forwards; will-change: transform, opacity; }
.loading { opacity: 1 !important; color: #c8aa6e !important; animation: pulse-icon 1s infinite; }
</style>
