<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { getCurrentWindow } from '@tauri-apps/api/window';

const appWindow = getCurrentWindow();
const version = ref('');
const rawNotes = ref('');
const status = ref<'idle' | 'downloading' | 'animating' | 'error'>('idle');
const errorMsg = ref('');
const downloadPct = ref(0);
const hasTotal = ref(false);
let accumulatedBytes = 0;
let totalBytes = 0;
let unlistenProgress: any = null;
let unlistenInstalling: any = null;

interface ReleaseChange { type: 'novo' | 'melhoria' | 'fix'; text: string; }
interface Release { version: string; date: string; changes: ReleaseChange[]; }

const currentRelease = ref<Release | null>(null);
const previousReleases = ref<Release[]>([]);

const notesFallbackLines = computed(() =>
  rawNotes.value.split('\n').map(l => l.replace(/^[-*•]\s*/, '').trim()).filter(l => l.length > 0)
);

async function loadChangelog(targetVersion: string) {
  let data: Release[] | null = null;
  try {
    const r = await fetch('https://raw.githubusercontent.com/SpellerBarbosa/riftsyncai/main/public/releases.json', { signal: AbortSignal.timeout(5000) });
    if (r.ok) data = await r.json();
  } catch (_) {}
  if (!data) {
    try { const r = await fetch('/releases.json'); if (r.ok) data = await r.json(); } catch (_) {}
  }
  if (!data || data.length === 0) return;
  const idx = targetVersion ? data.findIndex(r => r.version === targetVersion) : -1;
  currentRelease.value = idx >= 0 ? data[idx] : data[0];
  previousReleases.value = data.filter(r => r.version !== currentRelease.value?.version).slice(0, 4);
}

onMounted(async () => {
  const stored = localStorage.getItem('spellcoach_update_data');
  if (stored) {
    try { const p = JSON.parse(stored); version.value = p.version || ''; rawNotes.value = p.notes || ''; } catch (_) {}
  }
  await loadChangelog(version.value);

  unlistenProgress = await listen('update-progress', (e: any) => {
    const chunk: number = e.payload.chunk ?? 0;
    const total: number | null = e.payload.total ?? null;
    accumulatedBytes += chunk;
    if (total && total > 0) {
      hasTotal.value = true; totalBytes = total;
      downloadPct.value = Math.min(99, Math.round((accumulatedBytes / totalBytes) * 100));
    }
  });

  unlistenInstalling = await listen('update-installing', () => {
    downloadPct.value = 100;
    status.value = 'animating';
  });
});

onUnmounted(() => {
  if (unlistenProgress) unlistenProgress();
  if (unlistenInstalling) unlistenInstalling();
});

const closeWindow = () => appWindow.hide();

const installUpdate = async () => {
  status.value = 'downloading';
  accumulatedBytes = 0; totalBytes = 0; downloadPct.value = 0; hasTotal.value = false;
  invoke('download_and_install_update').catch((e: any) => {
    errorMsg.value = String(e);
    status.value = 'error';
  });
};

const typeLabel: Record<string, string> = { novo: 'NOVO', melhoria: 'MELHORIA', fix: 'FIX' };
const typeColor: Record<string, string> = { novo: '#4af076', melhoria: '#4ab4f0', fix: '#f0a84a' };
</script>

<template>
  <!-- Root: fixed inset-0 garante que preenche o viewport do WebView2 sem depender de parent heights -->
  <div class="fixed inset-0 flex flex-col overflow-hidden rounded-lg border border-[#c8aa6e] bg-[rgba(4,15,26,0.97)] text-[#f0e6d2] select-none"
       style="font-family:-apple-system,BlinkMacSystemFont,'Segoe UI',Roboto,Helvetica,Arial,sans-serif;box-shadow:0 0 30px rgba(0,0,0,0.8),inset 0 0 20px rgba(200,170,110,0.08)">

    <!-- Background radial gradient -->
    <div class="absolute inset-0 pointer-events-none"
         style="background:radial-gradient(circle at 50% 0%,rgba(10,36,56,0.85) 0%,rgba(1,10,19,1) 70%);z-index:0"></div>

    <!-- Nexus overlay (animação de instalação) -->
    <div v-if="status === 'animating'"
         class="absolute inset-0 flex items-center justify-center pointer-events-none"
         style="z-index:20">
      <div class="nexus-crystal"></div>
      <div class="nexus-shockwave absolute"></div>
      <div class="nexus-flash absolute inset-0"></div>
      <div class="nexus-text absolute left-0 right-0 text-center text-[12px] font-bold uppercase tracking-widest text-[#4ab4f0]"
           style="bottom:50px;text-shadow:0 0 10px rgba(74,180,240,0.8)">
        Instalando nova versão...
      </div>
    </div>

    <!-- Corpo principal -->
    <div class="relative flex flex-col flex-1 min-h-0 px-6 pt-5 transition-opacity duration-400"
         style="z-index:1"
         :style="status === 'animating' ? 'opacity:0;pointer-events:none' : 'opacity:1'">

      <!-- Cabeçalho -->
      <div class="shrink-0 text-center pb-4">
        <div class="w-12 h-12 rounded-xl flex items-center justify-center mx-auto mb-3 text-[22px]"
             style="background:linear-gradient(135deg,#c8aa6e,#7a5c29);box-shadow:0 4px 15px rgba(200,170,110,0.25)">
          ⚡
        </div>
        <h2 class="text-[18px] font-bold text-[#f0e6d2] m-0 mb-1 tracking-wide">Atualização Disponível</h2>
        <p class="text-[13px] font-semibold text-[#4af0a0] m-0">Versão {{ version }}</p>
      </div>

      <!-- Changelog: flex-1 + min-h-0 garante que cresce para preencher o espaço disponível -->
      <div class="flex-1 min-h-0 flex flex-col rounded overflow-hidden border border-[#c8aa6e]/25 bg-black/40">

        <!-- Header do changelog -->
        <div class="shrink-0 flex justify-between items-center px-3 py-1.5 text-[11px] font-bold uppercase tracking-widest text-[#c8aa6e] border-b border-[#c8aa6e]/15 bg-[#c8aa6e]/[0.08]">
          <span>📝 O que há de novo</span>
          <span v-if="currentRelease" class="text-[10px] font-normal normal-case tracking-normal text-[#3a3a3a]">
            {{ currentRelease.date }}
          </span>
        </div>

        <!-- Scroll area -->
        <div class="changelog-scroll flex-1 min-h-0 overflow-y-auto p-3 flex flex-col gap-1.5 text-[11px]">

          <template v-if="currentRelease">
            <div v-for="(c, i) in currentRelease.changes" :key="i" class="flex items-start gap-1.5">
              <span class="shrink-0 text-[8px] font-bold px-1 py-px border rounded-sm mt-px"
                    :style="{ color: typeColor[c.type], borderColor: typeColor[c.type] }">
                {{ typeLabel[c.type] }}
              </span>
              <span class="text-[#c8b89a] leading-snug">{{ c.text }}</span>
            </div>
          </template>

          <template v-else-if="notesFallbackLines.length">
            <div v-for="(line, i) in notesFallbackLines" :key="i" class="flex items-start gap-1.5">
              <span class="shrink-0 text-[8px] font-bold px-1 py-px border rounded-sm mt-px text-[#a09b8c] border-[#a09b8c]">•</span>
              <span class="text-[#c8b89a] leading-snug">{{ line }}</span>
            </div>
          </template>

          <div v-else class="text-[#5b5a56]">Pequenas correções e melhorias.</div>

          <template v-if="previousReleases.length">
            <div class="mt-2 mb-1.5 text-[9px] uppercase tracking-widest text-[#333] border-t border-[#1a1a1a] pt-2">
              Versões anteriores
            </div>
            <div v-for="rel in previousReleases" :key="rel.version" class="mb-1.5">
              <div class="text-[10px] font-bold text-[#3a3a3a] mb-1">
                v{{ rel.version }}
                <span class="text-[9px] font-normal text-[#2a2a2a] ml-1">{{ rel.date }}</span>
              </div>
              <div v-for="(c, i) in rel.changes" :key="i" class="flex items-start gap-1.5 opacity-45">
                <span class="shrink-0 text-[7px] font-bold px-1 py-px border rounded-sm mt-px"
                      :style="{ color: typeColor[c.type], borderColor: typeColor[c.type] }">
                  {{ typeLabel[c.type] }}
                </span>
                <span class="text-[#c8b89a] leading-snug">{{ c.text }}</span>
              </div>
            </div>
          </template>
        </div>
      </div>

      <!-- Ações: shrink-0 garante que os botões nunca somem -->
      <div class="shrink-0 flex gap-3 pt-3.5 pb-5">

        <!-- Estado: idle -->
        <template v-if="status === 'idle'">
          <button
            class="flex-1 h-11 rounded border border-[#3a3a3a] bg-transparent text-[#7a7570] text-[13px] font-bold uppercase tracking-wide cursor-pointer transition-colors hover:border-[#c8aa6e] hover:text-[#f0e6d2] hover:bg-[#c8aa6e]/[0.08]"
            @click="closeWindow">
            Lembrar Depois
          </button>
          <button
            class="flex-1 h-11 rounded border border-[#c8aa6e] text-[#f0e6d2] text-[13px] font-bold uppercase tracking-wide cursor-pointer relative overflow-hidden transition-shadow hover:shadow-[0_0_18px_rgba(200,170,110,0.4)]"
            style="background:linear-gradient(180deg,#1e282d 0%,#010a13 100%);box-shadow:0 0 10px rgba(200,170,110,0.15)"
            @click="installUpdate">
            Instalar e Reiniciar
            <span class="btn-sweep absolute top-0 left-[-100%] w-1/2 h-full pointer-events-none"
                  style="background:linear-gradient(90deg,transparent,rgba(255,255,255,0.15),transparent);transform:skewX(-20deg)"></span>
          </button>
        </template>

        <!-- Estado: downloading -->
        <template v-else-if="status === 'downloading'">
          <div class="w-full flex flex-col items-center gap-2.5">
            <div class="w-full h-1.5 bg-black/50 border border-[#c8aa6e]/25 rounded-full overflow-hidden relative">
              <div v-if="hasTotal"
                   class="h-full transition-[width] duration-200"
                   style="background:linear-gradient(90deg,#c8aa6e,#f0e84a)"
                   :style="{ width: downloadPct + '%' }"></div>
              <div v-else class="progress-indeterminate h-full"></div>
            </div>
            <span class="text-[11px] text-[#a09b8c] animate-pulse">
              {{ hasTotal ? `Baixando... ${downloadPct}%` : 'Baixando recursos hextech...' }}
            </span>
          </div>
        </template>

        <!-- Estado: error -->
        <template v-else-if="status === 'error'">
          <div class="w-full flex flex-col items-center gap-1.5">
            <span class="text-[18px]">⚠️</span>
            <span class="text-[11px] text-[#ff6b6b] text-center break-words">Falha ao atualizar: {{ errorMsg }}</span>
            <button
              class="mt-1 px-5 h-9 rounded border border-[#3a3a3a] bg-transparent text-[#7a7570] text-[11px] font-bold uppercase tracking-wide cursor-pointer hover:border-[#c8aa6e] hover:text-[#f0e6d2] transition-colors"
              @click="status = 'idle'">
              Tentar novamente
            </button>
          </div>
        </template>

      </div>
    </div>
  </div>
</template>

<style scoped>
/* Apenas keyframes e scrollbar — tudo o mais está em classes Tailwind */

/* Scrollbar do changelog */
.changelog-scroll::-webkit-scrollbar { width: 5px; }
.changelog-scroll::-webkit-scrollbar-track { background: transparent; }
.changelog-scroll::-webkit-scrollbar-thumb { background: rgba(200,170,110,0.35); border-radius: 3px; }

/* Sweep do botão primário */
.btn-sweep { animation: sweepAnim 3s infinite; }
@keyframes sweepAnim {
  0%   { left: -100%; }
  20%  { left: 200%; }
  100% { left: 200%; }
}

/* Progress bar indeterminado */
.progress-indeterminate {
  width: 100%;
  background: linear-gradient(90deg, transparent, #c8aa6e, transparent);
  background-size: 200% 100%;
  animation: indetermAnim 1.5s infinite linear;
}
@keyframes indetermAnim {
  0%   { background-position: 200% 0; }
  100% { background-position: -200% 0; }
}

/* Nexus explosion */
.nexus-crystal {
  width: 36px; height: 72px;
  background: linear-gradient(135deg, #4ab4f0, #0a4682);
  clip-path: polygon(50% 0%, 100% 25%, 100% 75%, 50% 100%, 0% 75%, 0% 25%);
  box-shadow: 0 0 30px #4ab4f0;
  animation: crystalFloat 1s ease-in-out infinite alternate, crystalShatter 2.5s forwards;
}
.nexus-shockwave {
  width: 10px; height: 10px;
  border-radius: 50%; border: 4px solid #4ab4f0;
  opacity: 0;
  animation: shockwaveAnim 2.5s forwards;
}
.nexus-flash { background: white; opacity: 0; animation: flashAnim 2.5s forwards; }
.nexus-text { animation: pulseAnim 1s infinite; }

@keyframes crystalFloat { 0% { transform: translateY(-4px); } 100% { transform: translateY(4px); } }
@keyframes crystalShatter {
  0%   { transform: scale(1) rotate(0deg);    filter: brightness(1); opacity: 1; }
  40%  { transform: scale(1.2) rotate(5deg);  filter: brightness(2); opacity: 1; }
  45%  { transform: scale(1.3) rotate(-5deg); filter: brightness(3); opacity: 1; }
  50%  { transform: scale(0) rotate(20deg);   opacity: 0; }
  100% { transform: scale(0); opacity: 0; }
}
@keyframes shockwaveAnim {
  0%,49% { transform: scale(1);  opacity: 0; }
  50%     { transform: scale(1);  opacity: 1; border-width: 20px; }
  70%     { transform: scale(40); opacity: 0; border-width: 1px; }
  100%    { transform: scale(40); opacity: 0; }
}
@keyframes flashAnim {
  0%,50% { opacity: 0; }
  55%,80%{ opacity: 1; }
  100%   { opacity: 0; }
}
@keyframes pulseAnim { 0%,100% { opacity: 0.6; } 50% { opacity: 1; } }
</style>
