<script setup lang="ts">
import { ref, onMounted, onUnmounted } from 'vue';
import { listen } from '@tauri-apps/api/event';
import { invoke } from '@tauri-apps/api/core';

const progress = ref(0);
const message = ref('Iniciando sincronização...');
const done = ref(false);

let unlisten: (() => void) | null = null;

onMounted(async () => {
  try {
    const currentState = await invoke<[number, string, boolean]>('get_sync_state');
    progress.value = currentState[0];
    message.value = currentState[1];
    done.value = currentState[2];
  } catch (e) {
    console.error('[SyncProgress] Falha ao recuperar estado:', e);
  }

  unlisten = await listen<{progress: number, message: string, done: boolean}>('sync-progress', (event) => {
    progress.value = event.payload.progress;
    message.value = event.payload.message;
    done.value = event.payload.done;
  });
});

onUnmounted(() => { if (unlisten) unlisten(); });
</script>

<template>
  <div class="w-[400px] h-[160px] flex items-center justify-center bg-transparent overflow-hidden p-2.5">
    <div class="w-full h-full bg-[#010a13] p-5 rounded border border-[#c8aa6e] shadow-[0_0_20px_rgba(0,0,0,0.8)] text-center flex flex-col justify-center">
      <h3 class="text-[#c8aa6e] uppercase tracking-widest mt-0 mb-2.5 text-[1.1rem] font-bold">Sincronização de Dados</h3>
      <p class="text-[#a09b8c] text-[0.85rem] mb-4 h-4 overflow-hidden text-ellipsis whitespace-nowrap">{{ message }}</p>

      <div class="relative w-full h-2 bg-[#1e2328] border border-[#3c3c41] rounded mt-1">
        <div class="h-full bg-gradient-to-r from-[#0ac8b9] to-[#005a82] rounded transition-[width] duration-200 ease-out relative z-[2]"
             :style="{ width: progress + '%' }"></div>
        <div class="absolute top-0 left-0 h-full bg-[#0ac8b9] blur-[6px] opacity-40 transition-[width] duration-200 ease-out z-[1] rounded"
             :style="{ width: progress + '%' }"></div>
        <span class="absolute right-0 -top-[18px] text-[#0ac8b9] font-bold text-[0.75rem]">{{ progress }}%</span>
      </div>

      <p v-if="done" class="text-[#0ac8b9] mt-2.5 font-bold text-[0.9rem] animate-pulse">Tudo pronto!</p>
    </div>
  </div>
</template>
