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
  <div class="w-[320px] min-h-[100px] flex flex-col p-2.5 rounded relative overflow-hidden shadow-[0_10px_30px_rgba(0,0,0,0.8)]"
       :class="{
         'border border-[rgba(120,90,40,0.5)] bg-gradient-to-br from-[#0a1428] to-[#010a13]': !rarity || rarity === 'common',
         'border border-[rgba(0,90,130,0.8)] bg-gradient-to-br from-[#0a1428] to-[#010a13]': rarity === 'rare',
         'border border-[#d000ff] bg-gradient-to-br from-[#0a1428] to-[#010a13]': rarity === 'epic',
         'border border-[#ffcc00] bg-gradient-to-br from-[#0a1428] to-[#010a13] shadow-[inset_0_0_15px_rgba(255,204,0,0.1),0_5px_15px_rgba(0,0,0,0.5)]': rarity === 'legendary',
       }">

    <!-- Header -->
    <div class="flex justify-between items-start mb-2 border-b border-[rgba(200,155,60,0.2)] pb-1.5">
      <div class="flex flex-col gap-0.5">
        <span class="text-[8px] font-extrabold text-[#a09b8c] tracking-[2px] uppercase">{{ title || 'DICA HEXTECH' }}</span>
        <span class="text-[13px] font-black text-[#f0e6d2] [text-shadow:0_0_10px_rgba(255,255,255,0.3)]">{{ frontText }}</span>
      </div>
      <div class="w-2 h-2 rounded-full mt-1 shrink-0"
           :class="{
             'bg-[#785a28]': !rarity || rarity === 'common',
             'bg-[#005a82] shadow-[0_0_8px_#005a82]': rarity === 'rare',
             'bg-[#d000ff] shadow-[0_0_8px_#d000ff]': rarity === 'epic',
             'bg-[#ffcc00] shadow-[0_0_8px_#ffcc00]': rarity === 'legendary',
           }"></div>
    </div>

    <!-- Body -->
    <div class="flex gap-3 flex-1">
      <div v-if="imageUrl" class="w-[45px] h-[45px] border border-[rgba(200,155,60,0.3)] bg-black shrink-0 overflow-hidden">
        <img :src="imageUrl" alt="champion" class="w-full h-full object-cover" />
      </div>
      <div class="flex-1 flex flex-col justify-center">
        <div class="text-[11px] leading-[1.4] text-[#cdbe91] font-medium" v-html="backText?.replace(/\n/g, '<br>')"></div>
      </div>
    </div>

    <!-- Scanline footer -->
    <div class="absolute bottom-0 left-0 w-full h-0.5 bg-[rgba(200,155,60,0.1)] overflow-hidden">
      <div class="w-full h-full scanline"></div>
    </div>
  </div>
</template>

<style scoped>
.scanline {
  background: linear-gradient(90deg, transparent, var(--accent-gold, #c89b3c), transparent);
  animation: scan 3s linear infinite;
  will-change: transform;
  transform: translateZ(0);
}
@keyframes scan {
  from { transform: translateX(-100%) translateZ(0); }
  to   { transform: translateX(100%) translateZ(0); }
}
</style>
