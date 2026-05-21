<script setup lang="ts">
import { ref, onMounted, onUnmounted } from 'vue';
import { listen } from '@tauri-apps/api/event';
import { invoke } from '@tauri-apps/api/core';

const progress = ref(0);
const message = ref('Iniciando sincronização...');
const done = ref(false);

let unlisten: (() => void) | null = null;

onMounted(async () => {
  console.log('[SyncProgress] Monitor montado, escutando evento sync-progress...');
  
  // Query active progress state from Rust backend on load to eliminate race conditions
  try {
    const currentState = await invoke<[number, string, boolean]>('get_sync_state');
    console.log('[SyncProgress] Estado inicial recuperado via IPC:', currentState);
    progress.value = currentState[0];
    message.value = currentState[1];
    done.value = currentState[2];
  } catch (e) {
    console.error('[SyncProgress] Falha ao recuperar estado de sincronização:', e);
  }

  unlisten = await listen<{progress: number, message: string, done: boolean}>('sync-progress', (event) => {
    console.log('[SyncProgress] Evento recebido no componente:', event.payload);
    progress.value = event.payload.progress;
    message.value = event.payload.message;
    done.value = event.payload.done;
  });
});

onUnmounted(() => {
  if (unlisten) unlisten();
});
</script>

<template>
  <div class="sync-container">
    <div class="sync-card">
      <div class="content">
        <h3>Sincronização de Dados</h3>
        <p class="message">{{ message }}</p>
        
        <div class="progress-container">
          <div class="progress-bar" :style="{ width: progress + '%' }"></div>
          <div class="progress-glow" :style="{ width: progress + '%' }"></div>
          <span class="percentage">{{ progress }}%</span>
        </div>
        
        <p v-if="done" class="done-text">Tudo pronto!</p>
      </div>
    </div>
  </div>
</template>

<style scoped>
.sync-container {
  width: 400px;
  height: 160px;
  display: flex;
  align-items: center;
  justify-content: center;
  background: transparent;
  overflow: hidden;
  padding: 10px;
}

.sync-card {
  width: 100%;
  height: 100%;
  background: #010a13;
  padding: 20px;
  border-radius: 4px;
  border: 1px solid #c8aa6e;
  box-shadow: 0 0 20px rgba(0, 0, 0, 0.8);
  text-align: center;
  display: flex;
  flex-direction: column;
  justify-content: center;
}

h3 {
  color: #c8aa6e;
  font-family: 'BeaufortforLOL', serif;
  margin-top: 0;
  margin-bottom: 10px;
  text-transform: uppercase;
  letter-spacing: 2px;
  font-size: 1.1rem;
}

.message {
  color: #a09b8c;
  font-size: 0.85rem;
  margin-bottom: 15px;
  height: 1rem;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.progress-container {
  position: relative;
  width: 100%;
  height: 8px;
  background: #1e2328;
  border: 1px solid #3c3c41;
  border-radius: 4px;
  margin-top: 5px;
}

.progress-bar {
  height: 100%;
  background: linear-gradient(90deg, #0ac8b9, #005a82);
  border-radius: 4px;
  transition: width 0.2s ease-out;
  position: relative;
  z-index: 2;
}

.progress-glow {
  position: absolute;
  top: 0;
  left: 0;
  height: 100%;
  background: #0ac8b9;
  filter: blur(6px);
  opacity: 0.4;
  transition: width 0.2s ease-out;
  z-index: 1;
}

.percentage {
  position: absolute;
  right: 0;
  top: -18px;
  color: #0ac8b9;
  font-weight: bold;
  font-size: 0.75rem;
}

.done-text {
  color: #0ac8b9;
  margin-top: 10px;
  font-weight: bold;
  font-size: 0.9rem;
  animation: pulse 1s infinite alternate;
}

@keyframes pulse {
  from { opacity: 0.6; }
  to { opacity: 1; }
}
</style>
