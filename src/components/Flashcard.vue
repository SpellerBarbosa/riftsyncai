<script setup lang="ts">
defineProps<{
  title?: string;
  frontText?: string;
  backText?: string;
  imageUrl?: string;
  rarity?: 'common' | 'rare' | 'epic' | 'legendary';
}>();
</script>

<template>
  <div class="tactical-card rarity-border" :class="rarity || 'common'">
    <div class="card-header">
      <div class="title-group">
        <span class="label-mini">{{ title || 'DICA HEXTECH' }}</span>
        <span class="focus-badge">{{ frontText }}</span>
      </div>
      <div class="rarity-dot" :class="rarity"></div>
    </div>
    
    <div class="card-body">
      <div v-if="imageUrl" class="card-image">
        <img :src="imageUrl" alt="champion" />
      </div>
      <div class="card-content">
        <div class="reason-text" v-html="backText?.replace(/\n/g, '<br>')"></div>
      </div>
    </div>
    
    <div class="card-footer">
      <div class="scanline"></div>
    </div>
  </div>
</template>

<style scoped>
.tactical-card {
  width: 320px;
  min-height: 100px;
  background: linear-gradient(135deg, #0a1428 0%, #010a13 100%);
  border: 1px solid var(--accent-gold);
  border-radius: 4px;
  display: flex;
  flex-direction: column;
  padding: 10px;
  position: relative;
  overflow: hidden;
  box-shadow: 0 10px 30px rgba(0,0,0,0.8);
}

.card-header {
  display: flex;
  justify-content: space-between;
  align-items: flex-start;
  margin-bottom: 8px;
  border-bottom: 1px solid rgba(200, 155, 60, 0.2);
  padding-bottom: 6px;
}

.title-group {
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

.focus-badge {
  font-size: 13px;
  font-weight: 900;
  color: #f0e6d2;
  text-shadow: 0 0 10px rgba(255, 255, 255, 0.3);
}

.rarity-dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  margin-top: 4px;
}

.rarity-dot.common { background: #785a28; }
.rarity-dot.rare { background: #005a82; box-shadow: 0 0 8px #005a82; }
.rarity-dot.epic { background: #d000ff; box-shadow: 0 0 8px #d000ff; }
.rarity-dot.legendary { background: #ffcc00; box-shadow: 0 0 8px #ffcc00; }

.card-body {
  display: flex;
  gap: 12px;
  flex: 1;
}

.card-image {
  width: 45px;
  height: 45px;
  border: 1px solid rgba(200, 155, 60, 0.3);
  background: #000;
  flex-shrink: 0;
}

.card-image img {
  width: 100%;
  height: 100%;
  object-fit: cover;
}

.card-content {
  flex: 1;
  display: flex;
  flex-direction: column;
  justify-content: center;
}

.reason-text {
  font-size: 11px;
  line-height: 1.4;
  color: #cdbe91;
  font-weight: 500;
}

.card-footer {
  position: absolute;
  bottom: 0;
  left: 0;
  width: 100%;
  height: 2px;
  background: rgba(200, 155, 60, 0.1);
}

.scanline {
  width: 100%;
  height: 100%;
  background: linear-gradient(90deg, transparent, var(--accent-gold), transparent);
  animation: scan 3s linear infinite;
  will-change: transform; /* Força compositing na GPU, evita repaint da CPU */
  transform: translateZ(0); /* Cria stacking context isolado */
}

@keyframes scan {
  from { transform: translateX(-100%) translateZ(0); }
  to { transform: translateX(100%) translateZ(0); }
}

/* Rarity Borders */
.rarity-border.common { border-color: rgba(120, 90, 40, 0.5); }
.rarity-border.rare { border-color: rgba(0, 90, 130, 0.8); }
.rarity-border.epic { border-color: #d000ff; }
.rarity-border.legendary { 
  border-color: #ffcc00; 
  box-shadow: inset 0 0 15px rgba(255, 204, 0, 0.1), 0 5px 15px rgba(0,0,0,0.5);
}
</style>
