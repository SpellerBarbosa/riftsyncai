<script setup lang="ts">
import { ref, onMounted, onUnmounted, computed } from 'vue';
import { listen } from '@tauri-apps/api/event';

const closeWindow = async () => {
  try {
    const { emit } = await import('@tauri-apps/api/event');
    await emit('hide-rune-overlay');
  } catch (e) {
    console.error("Erro ao emitir hide-rune-overlay:", e);
  }
};

// Dynamic state from the backend
const championName = ref('Campeão');
const primaryTreeName = ref('Precisão');
const primaryTreeId = ref(8000);
const secondaryTreeName = ref('Dominação');
const secondaryTreeId = ref(8100);
const keystoneName = ref('Conquistador');
const keystoneId = ref(8010);
const runesList = ref<number[]>([]);
const shardsList = ref<number[]>([]);

const activeColor = ref('#C89B3C');

import {
  RUNE_COLORS,
  TREE_STRUCTURES,
  SHARDS_ROWS,
  getFullUrl,
  getTreeHeaderIcon,
  normalizeRuneId
} from './runesData';


const primaryTreeObj = computed(() => TREE_STRUCTURES[primaryTreeId.value] || TREE_STRUCTURES[8000]);
const secondaryTreeObj = computed(() => TREE_STRUCTURES[secondaryTreeId.value] || TREE_STRUCTURES[8100]);

let unlistenUpdate: any;

onMounted(async () => {
  unlistenUpdate = await listen('update-rune-overlay-content', (event: any) => {
    const data = event.payload;
    console.log('[RuneOverlay] update-rune-overlay-content recebido:', data);
    
    championName.value = data.champion_name;
    primaryTreeName.value = data.primary_tree_name;
    primaryTreeId.value = data.primary_tree_id;
    secondaryTreeName.value = data.secondary_tree_name;
    secondaryTreeId.value = data.secondary_tree_id;
    keystoneName.value = data.keystone_name;
    keystoneId.value = data.keystone_id;
    // Normalize IDs: map alias/renamed rune IDs to their canonical visual equivalents
    runesList.value = (data.runes || []).map((id: number) => normalizeRuneId(id));
    shardsList.value = data.shards || [];

    
    activeColor.value = RUNE_COLORS[data.primary_tree_id] || '#C89B3C';
  });

  const { emit } = await import('@tauri-apps/api/event');
  emit('request-rune-overlay-content');
  window.addEventListener('keydown', onKey);
});

const onKey = (e: KeyboardEvent) => { if (e.key === 'Escape') closeWindow(); };

onUnmounted(() => {
  if (unlistenUpdate) unlistenUpdate();
  window.removeEventListener('keydown', onKey);
});
</script>

<template>
  <div class="rune-card glass" :style="{ '--accent-color': activeColor }">
    <div class="card-header">
      <div class="header-details">
        <span class="label-mini">PÁGINA DE RUNAS IA</span>
        <h2 class="champ-title" :style="{ color: activeColor }">{{ championName }}</h2>
      </div>
      <div class="header-right-controls">
        <button class="close-overlay-btn" @click="closeWindow" title="Fechar Overlay">×</button>
        <div class="glow-tree-indicator" :style="{ background: activeColor }"></div>
      </div>
    </div>

    <!-- Main Assembled Columns Grid -->
    <div class="card-body">
      <!-- COLUMN 1: Primary Tree -->
      <div class="tree-column">
        <div class="column-header">
          <img :src="getTreeHeaderIcon(primaryTreeId)" class="tree-header-icon" />
          <span class="tree-title" :style="{ color: activeColor }">{{ primaryTreeName }}</span>
        </div>

        <div class="rune-path-grid">
          <!-- Keystone row -->
          <div class="path-row keystones">
            <div 
              v-for="k in primaryTreeObj.keystones" 
              :key="k.id"
              class="rune-circle keystone"
              :class="{ 'active animate-glow': runesList.includes(k.id), 'inactive': !runesList.includes(k.id) }"
              :style="{ '--glow-color': activeColor }"
              :title="k.name"
            >
              <img :src="getFullUrl(k.icon)" class="rune-icon" />
            </div>
          </div>

          <!-- Sub-runes rows -->
          <div 
            v-for="(row, rowIdx) in primaryTreeObj.rows" 
            :key="rowIdx" 
            class="path-row sub-runes"
          >
            <div 
              v-for="r in row" 
              :key="r.id"
              class="rune-circle sub-rune"
              :class="{ 'active': runesList.includes(r.id), 'inactive': !runesList.includes(r.id) }"
              :style="{ '--glow-color': activeColor }"
              :title="r.name"
            >
              <img :src="getFullUrl(r.icon)" class="rune-icon" />
            </div>
          </div>
        </div>
      </div>

      <!-- COLUMN 2: Secondary Tree & Shards -->
      <div class="tree-column right">
        <!-- Secondary Tree header -->
        <div class="column-header">
          <img :src="getTreeHeaderIcon(secondaryTreeId)" class="tree-header-icon sm" />
          <span class="tree-title secondary">{{ secondaryTreeName }}</span>
        </div>

        <div class="rune-path-grid secondary-tree">
          <!-- Sub-runes rows (Secondary has no keystones!) -->
          <div 
            v-for="(row, rowIdx) in secondaryTreeObj.rows" 
            :key="rowIdx" 
            class="path-row sub-runes secondary-row"
          >
            <div 
              v-for="r in row" 
              :key="r.id"
              class="rune-circle sub-rune sm"
              :class="{ 'active': runesList.includes(r.id), 'inactive': !runesList.includes(r.id) }"
              :style="{ '--glow-color': RUNE_COLORS[secondaryTreeId] || '#cdbe91' }"
              :title="r.name"
            >
              <img :src="getFullUrl(r.icon)" class="rune-icon" />
            </div>
          </div>
        </div>

        <!-- Shards mods (Atributos) at the bottom-right -->
        <div class="shards-box">
          <span class="shards-title">Atributos</span>
          <div class="shards-path-grid">
            <div 
              v-for="(row, rowIdx) in SHARDS_ROWS" 
              :key="rowIdx"
              class="path-row shards-row"
            >
              <div 
                v-for="s in row" 
                :key="s.id"
                class="shard-circle"
                :class="{ 'active': shardsList[rowIdx] === s.id, 'inactive': shardsList[rowIdx] !== s.id }"
                :title="s.name"
              >
                <img :src="getFullUrl(s.icon)" class="shard-icon" />
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>

    <!-- Hextech Footer -->
    <div class="card-footer">
      <div class="footer-bar animate-glow" :style="{ background: activeColor }"></div>
      <span class="brand-text">SPELL COACH IA • PÁGINA MONTADA</span>
    </div>
  </div>
</template>

<style scoped>
.rune-card {
  width: 460px;
  height: 380px;
  background: linear-gradient(135deg, rgba(3, 8, 16, 0.98) 0%, rgba(1, 4, 8, 0.99) 100%);
  border: 1px solid var(--accent-color);
  border-radius: 8px;
  display: flex;
  flex-direction: column;
  padding: 12px 16px;
  position: relative;
  overflow: hidden;
  box-shadow: 0 15px 35px rgba(0, 0, 0, 0.9), inset 0 0 20px rgba(0,0,0,0.5);
  transition: all 0.5s cubic-bezier(0.16, 1, 0.3, 1);
}

.card-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 12px;
  border-bottom: 1px solid rgba(255, 255, 255, 0.08);
  padding-bottom: 8px;
}

.header-right-controls {
  display: flex;
  align-items: center;
  gap: 10px;
}

.close-overlay-btn {
  background: none;
  border: none;
  color: #cdbe91;
  font-size: 20px;
  font-weight: 300;
  cursor: pointer;
  opacity: 0.6;
  transition: all 0.2s ease;
  line-height: 1;
  padding: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  width: 20px;
  height: 20px;
  border-radius: 4px;
}

.close-overlay-btn:hover {
  opacity: 1;
  color: #ff4e4e;
  background: rgba(255, 78, 78, 0.1);
}

.header-details {
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.label-mini {
  font-size: 8px;
  font-weight: 800;
  color: #a09b8c;
  letter-spacing: 2px;
  text-transform: uppercase;
}

.champ-title {
  font-size: 18px;
  font-weight: 900;
  margin: 0;
  letter-spacing: 0.5px;
  text-transform: uppercase;
  text-shadow: 0 0 12px rgba(255, 255, 255, 0.1);
}

.glow-tree-indicator {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  box-shadow: 0 0 10px var(--accent-color);
}

/* Core grid display: two columns */
.card-body {
  flex: 1;
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 20px;
  align-items: start;
}

.tree-column {
  display: flex;
  flex-direction: column;
  gap: 10px;
}

.column-header {
  display: flex;
  align-items: center;
  gap: 8px;
  border-bottom: 1px solid rgba(255, 255, 255, 0.04);
  padding-bottom: 4px;
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
  font-size: 11px;
  font-weight: 800;
  text-transform: uppercase;
  letter-spacing: 1px;
}

.tree-title.secondary {
  color: #a09b8c;
}

/* Rune visual paths */
.rune-path-grid {
  display: flex;
  flex-direction: column;
  gap: 12px;
  align-items: center;
  padding: 6px 0;
  background: rgba(255,255,255,0.01);
  border-radius: 6px;
  border: 1px solid rgba(255,255,255,0.02);
}

.path-row {
  display: flex;
  justify-content: center;
  gap: 14px;
  width: 100%;
}

.path-row.keystones {
  gap: 10px;
}

.path-row.shards-row {
  gap: 12px;
}

/* Perfect interactive circles matching league client */
.rune-circle {
  width: 28px;
  height: 28px;
  border-radius: 50%;
  display: flex;
  align-items: center;
  justify-content: center;
  background: rgba(0, 0, 0, 0.65);
  border: 1px solid rgba(255, 255, 255, 0.1);
  transition: all 0.25s cubic-bezier(0.4, 0, 0.2, 1);
  cursor: pointer;
  position: relative;
}

.rune-circle.keystone {
  width: 36px;
  height: 36px;
}

.rune-circle.sm {
  width: 24px;
  height: 24px;
}

.rune-icon {
  width: 80%;
  height: 80%;
  object-fit: contain;
  transition: all 0.25s ease;
}

/* Visual highlights for ACTIVE choices */
.rune-circle.active {
  border-color: var(--glow-color);
  box-shadow: 0 0 10px var(--glow-color);
  background: rgba(0, 0, 0, 0.85);
  transform: scale(1.12);
  z-index: 2;
}

.rune-circle.active.keystone {
  box-shadow: 0 0 15px var(--glow-color);
  border-width: 1.5px;
}

.rune-circle.active .rune-icon {
  filter: none;
  transform: scale(1.05);
}

/* Grayscale/faded fallback for INACTIVE options */
.rune-circle.inactive {
  opacity: 0.15;
  filter: grayscale(1) contrast(0.85);
  background: rgba(0, 0, 0, 0.4);
}

.rune-circle.inactive:hover {
  opacity: 0.35;
  filter: grayscale(0.5);
}

/* Shards at bottom right */
.shards-box {
  margin-top: 14px;
  padding: 8px 12px;
  background: rgba(255, 255, 255, 0.02);
  border-radius: 6px;
  border: 1px solid rgba(255, 255, 255, 0.03);
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.shards-title {
  font-size: 8px;
  font-weight: 800;
  text-transform: uppercase;
  color: #a09b8c;
  letter-spacing: 1.5px;
  border-bottom: 1px solid rgba(255,255,255,0.03);
  padding-bottom: 4px;
}

.shards-path-grid {
  display: flex;
  flex-direction: column;
  gap: 8px;
  align-items: center;
}

.shard-circle {
  width: 16px;
  height: 16px;
  border-radius: 50%;
  background: rgba(0,0,0,0.7);
  border: 1px solid rgba(255,255,255,0.1);
  display: flex;
  align-items: center;
  justify-content: center;
  transition: all 0.2s ease;
}

.shard-icon {
  width: 70%;
  height: 70%;
  object-fit: contain;
}

.shard-circle.active {
  border-color: #cdbe91;
  box-shadow: 0 0 6px #cdbe91;
  transform: scale(1.15);
  background: rgba(0,0,0,0.9);
}

.shard-circle.inactive {
  opacity: 0.2;
  filter: grayscale(1);
}

/* Hextech footer details */
.card-footer {
  margin-top: 12px;
  position: relative;
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding-top: 8px;
}

.footer-bar {
  position: absolute;
  top: 0;
  left: 0;
  width: 100%;
  height: 1px;
  opacity: 0.3;
}

.brand-text {
  font-size: 7px;
  font-weight: 800;
  color: #cdbe91;
  opacity: 0.5;
  letter-spacing: 1px;
}
</style>
