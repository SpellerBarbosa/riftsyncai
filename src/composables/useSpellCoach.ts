/*
===============================================================================
                    SPELL COACH IA - COMPOSABLE FRONTEND
===============================================================================
Este arquivo é um Vue 3 Composable (useSpellCoach).

Para um estudante de programação:
* O que é um Composable?
  No Vue 2, o código de scripts era organizado de forma engessada (Options API). 
  No Vue 3, com a Composition API, criamos "Composables" que são funções que encapsulam
  estado reativo (refs, computeds) e comportamento (ciclo de vida, watchers, funções).
  Isso nos permite retirar centenas de linhas de lógica de dentro de arquivos .vue 
  e movê-las para arquivos TypeScript limpos (.ts).
* Por que isso é Clean Code?
  Separamos a Apresentação (o HTML/CSS em App.vue) da Lógica de Negócios (este arquivo).
  Isso facilita testes de lógica, melhora a legibilidade e permite reuso!
===============================================================================
*/

import { ref, onMounted, onUnmounted, watch } from "vue";
import { listen, emit } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { WebviewWindow } from "@tauri-apps/api/webviewWindow";
import { useVoiceCoach } from "./useVoiceCoach";

interface DismissCondition {
  type: 'fixed' | 'clear_step_gt' | 'level_gt' | 'game_time_gte' | 'inventory_change' | 'skill_leveled' | 'fallback';
  target?: number;
  max_ms?: number;
  snapshot?: { q: number; w: number; e: number; r: number };
}

interface GameState {
  game_time: number;
  level: number;
  gold: number;
  cs: number;
  clear_step: number;
  inventory: number[];
  abilities: { q: number; w: number; e: number; r: number };
}

interface QueuedTip {
  title: string;
  frontText: string;
  backText: string;
  rarity: 'common' | 'rare' | 'epic' | 'legendary' | 'mythic';
  priority: number;
  timestamp: number;
  dismiss?: DismissCondition;
  tipCategory?: string;
}

// NOTA: tipQueue e currentActiveTip foram movidos para dentro de useSpellCoach()
// para evitar estado compartilhado entre múltiplas janelas/webviews do Tauri.
// Ver Bug #1 no diagnóstico de performance.

const getRarityPriority = (rarity: string, title: string): number => {
  const t = (title || "").toUpperCase();
  if (t.includes("ALERTA") || t.includes("GANK") || t.includes("PERIGO") || t.includes("DANGER") || t.includes("CUIDADO")) {
    return 5; // Highest priority
  }
  const r = (rarity || "").toLowerCase();
  if (r === "mythic") return 4;
  if (r === "legendary") return 3;
  if (r === "epic") return 2;
  if (r === "rare") return 1;
  return 0; // common
};

const getSummonerNickname = (fullName: string): string => {
  if (!fullName || fullName === "---" || fullName === "Invocador") return "mano";
  const parts = fullName.split('#');
  return parts[0].trim();
};

const customizeTextWithNickname = (text: string, nickname: string, title: string): string => {
  if (!text || !nickname) return text;
  
  let customized = text;
  const upperTitle = (title || "").toUpperCase();
  
  // 1. Alertas de perigo, gank ou mapa: insere "[Nick]! " de forma enfática no início
  const isUrgent = upperTitle.includes("ALERTA") || 
                   upperTitle.includes("PERIGO") || 
                   upperTitle.includes("RICO") || 
                   upperTitle.includes("VIDA CRÍTICA") || 
                   upperTitle.includes("SEM MANA") || 
                   upperTitle.includes("INIMIGO FED") || 
                   upperTitle.includes("MAPA");
                   
  if (isUrgent) {
    if (!customized.toLowerCase().includes(nickname.toLowerCase())) {
      customized = `${nickname}! ${customized}`;
    }
    return customized;
  }
  
  // 2. Remove prefixos técnicos de rotas ou categorias e substitui por "[Nick], "
  // Ex: "JG: faça isso" -> "[Nick], faça isso"
  // Ex: "Dica Micro (Selva-Farm): faça isso" -> "[Nick], faça isso"
  const roleRegex = /^(JG|Mid|ADC|Sup|Top):\s*/i;
  const categoryRegex = /^(Dica Micro|Macro|Alerta de Farm|Atenção ao Farm|Atenção de Defesa)\s*\([^)]+\):\s*/i;
  const categoryRegexNoParens = /^(Alerta de Farm|Atenção ao Farm|Atenção de Defesa):\s*/i;
  
  if (categoryRegex.test(customized)) {
    customized = customized.replace(categoryRegex, `${nickname}, `);
  } else if (categoryRegexNoParens.test(customized)) {
    customized = customized.replace(categoryRegexNoParens, `${nickname}, `);
  } else if (roleRegex.test(customized)) {
    customized = customized.replace(roleRegex, `${nickname}, `);
  } else {
    if (!customized.toLowerCase().includes(nickname.toLowerCase())) {
      customized = `${nickname}, ${customized.charAt(0).toLowerCase()}${customized.slice(1)}`;
    }
  }
  
  return customized;
};

export function useSpellCoach() {
  const { speak, stop: stopVoice, voiceEnabled, kokoroStatus } = useVoiceCoach();

  // ---------------------------------------------------------------------------
  // 1. ESTADO REATIVO (Reactive State)
  // ---------------------------------------------------------------------------
  // Para o estudante: O 'ref()' cria uma referência reativa. Quando o valor de uma ref 
  // muda no TypeScript, o Vue atualiza automaticamente a tela em milissegundos!
  
  const appWindow = getCurrentWindow(); // Obtém a instância da janela webview atual do Tauri
  const windowLabel = ref(appWindow.label); // Armazena o identificador único da janela (ex: 'main', 'build', 'settings')
  
  const settingsLoading = ref(false); // Flag visual para feedback de carregamento da tela de config
  const dataViewerLoading = ref(false); // Flag visual para feedback de carregamento da tela de estatísticas
  
  const isLive = ref(false); // Indica se o LCU do League of Legends está ativo e conectado
  const summonerName = ref("---"); // Nome completo da conta do jogador (ex: Faker#KR1)
  const lcuStatus = ref("OFFLINE"); // Status visual detalhado do cliente LCU (Connected, Offline)
  const gameFlowState = ref("IDLE"); // Estado do fluxo da partida (Champ Select, In Game, End of Game)
  
  const activeChampion = ref<string | null>(null); // Campeão selecionado ou jogado ativamente
  const isFetchingTips = ref(false); // Semáforo para impedir consultas concorrentes repetidas à IA
  const lastAutoTacticalTipChamp = ref<string | null>(null); // Evita repetir a mesma dica OpenRouter para o mesmo campeão
  const runeOverlayData = ref<any>({}); // Objeto de dados das runas e feitiços que recebemos do backend Rust
  const playerRole = ref("MID"); // Rota atual em que o jogador está atuando
  // Controla qual motor de dicas está ativo: true = Groq (IA Nuvem), false = Procedural (Rust bridge)
  // Quando Groq está ativo, os emits procedurais do bridge Rust são silenciados in-game.
  const groqEnabled = ref(false);
  // true quando o Groq falhou na última chamada (tokens/rate-limit/rede esgotados).
  // Libera automaticamente as dicas procedurais do Rust bridge como fallback.
  const groqExhausted = ref(false);
  
  // BUG FIX #1: tipQueue e currentActiveTip agora são LOCAIS à instância da janela main.
  // Anteriormente eram globais (módulo), causando que a janela 'flashcard' e 'main' 
  // compartilhassem a mesma fila — chamadas paralelas de showNextTip() eram o resultado.
  const tipQueue = ref<QueuedTip[]>([]);
  const currentActiveTip = ref<QueuedTip | null>(null);

  // Estados específicos para o controle do widget de dicas (Flashcard)
  const isExitingFlashcard = ref(false);
  const flashcardKey = ref(0);
  const flashcardData = ref({
    title: "Dica do Coach",
    frontText: "Analisando draft...",
    backText: "Aguarde a análise...",
    rarity: "epic" as 'common' | 'rare' | 'epic' | 'legendary'
  });

  // Estado do pós-jogo
  const postGameReport = ref<any>(null);
  const postGameLoading = ref(false);

  // Estado do pré-jogo (loading screen)
  const preGameReport = ref<any>(null);

  // Chave numérica Riot do adversário real de rota (preenchido no Champ Select)
  const laneOpponentKey = ref<number | null>(null);

  // Estado do ward map
  const wardMapData = ref({
    champion: "",
    role: "MID",
    phase: "early",
    teamSide: "blue",
    wards: [] as Array<{ x: number; y: number; priority: number }>,
    gameTime: 0,
    objective: "",        // "Dragão", "Barão", etc. — vazio = card genérico
    objectiveEmoji: "",   // "🐉", "💜", etc.
    secondsToSpawn: 0,    // segundos restantes para o spawn do objetivo
  });

  // ---------------------------------------------------------------------------
  // 2. FUNÇÕES DE TRADUÇÃO E INFRAESTRUTURA
  // ---------------------------------------------------------------------------

  /// Normaliza os nomes das rotas recebidos da API LCU para as nomenclaturas do banco de dados.
  const getNormalizedRole = (rawRole: string) => {
    if (!rawRole) return "MID";
    const r = rawRole.toUpperCase();
    if (r === "BOTTOM") return "ADC";
    if (r === "UTILITY") return "SUPPORT";
    return r;
  };

  // ---------------------------------------------------------------------------
  // 3. GERENCIADORES DE JANELAS MULTI-WEBVIEW (Tauri WebviewWindow)
  // ---------------------------------------------------------------------------
  // Para o estudante (Arquitetura Tauri):
  // O aplicativo usa sub-janelas transparentes que parecem "widgets flutuantes" no Windows.
  // Instanciamos novas janelas WebviewWindow passando opções de transparência, decoração (decorations: false
  // esconde as barras de título padrão do Windows) e centralização.

  /// Abre ou foca na sub-janela de Configurações.
  const openSettings = async () => {
    settingsLoading.value = true;
    try {
      let win = await WebviewWindow.getByLabel("settings");
      if (!win) {
        win = new WebviewWindow("settings", {
          url: "index.html",
          title: "Configurações",
          width: 500,
          height: 600,
          transparent: true,
          decorations: false,
          center: true,
        });
      }
      if (win) {
        await win.show();
        await win.unminimize();
        await win.setFocus();
      }
    } catch (e) {
      console.error("Erro ao abrir configurações:", e);
    } finally {
      setTimeout(() => { settingsLoading.value = false; }, 1000);
    }
  };

  /// Abre ou foca na sub-janela do Visualizador de Estatísticas e Históricos IA.
  const openDataViewer = async () => {
    dataViewerLoading.value = true;
    try {
      let win = await WebviewWindow.getByLabel("data-viewer");
      if (!win) {
        win = new WebviewWindow("data-viewer", {
          url: "index.html",
          title: "Perfil & Estatísticas IA",
          width: 800,
          height: 620,
          transparent: true,
          decorations: false,
          center: true,
        });
      }
      if (win) {
        await win.show();
        await win.unminimize();
        await win.setFocus();
      }
    } catch (e) {
      console.error("Erro ao abrir visualizador de dados:", e);
    } finally {
      setTimeout(() => { dataViewerLoading.value = false; }, 1000);
    }
  };

  /// Abre ou fecha a janela do Ward Map.
  /// Cria a janela dinamicamente na primeira vez — sem setFocus() para não pausar o jogo.
  /// Posicionada no canto superior direito para não cobrir o minimap do LoL (bottom-right).
  const toggleWardMap = async (forceState?: boolean) => {
    try {
      let win = await WebviewWindow.getByLabel("ward-map");
      if (!win) {
        // Detecta resolução da tela para posicionar no canto superior direito
        const screenW = window.screen.width || 1920;
        win = new WebviewWindow("ward-map", {
          url: "index.html",
          title: "Ward Map",
          width: 280,
          height: 360,
          x: screenW - 288,
          y: 60,
          transparent: true,
          decorations: false,
          alwaysOnTop: true,
          skipTaskbar: true,
          visible: false,
        });
        // Aguarda a janela carregar antes de mostrar/emitir dados
        await new Promise(r => setTimeout(r, 400));
      }
      const isVisible = await win.isVisible();
      const shouldShow = forceState !== undefined ? forceState : !isVisible;
      if (shouldShow) {
        await win.show();
      } else if (isVisible) {
        await win.hide();
      }
    } catch (e) {
      console.error("[Toggle] Erro no toggleWardMap:", e);
    }
  };

  /// Abre ou fecha o overlay flutuante de Runas Recomendadas sobre o jogo.
  /// PERFORMANCE: sem setFocus() para não roubar o foco do jogo.
  const toggleRuneOverlay = async (forceState?: boolean) => {
    try {
      let win = await WebviewWindow.getByLabel("rune-overlay");
      if (!win) {
        win = new WebviewWindow("rune-overlay", {
          url: "index.html",
          title: "Spell Coach Rune Overlay",
          width: 480,
          height: 400,
          transparent: true,
          decorations: false,
          alwaysOnTop: true, // Garante que a janela sobreponha o League of Legends em borderless
          skipTaskbar: true,
          visible: false,
        });
        await new Promise(r => setTimeout(r, 200));
      }

      const isVisible = await win.isVisible();
      const shouldShow = forceState !== undefined ? forceState : !isVisible;

      if (shouldShow) {
        await win.show();
        // NÃO chamamos setFocus() — isso pausaria o jogo!
      } else if (isVisible) {
        await win.hide();
      }
    } catch (e) {
      console.error("[Toggle] Erro no toggleRuneOverlay:", e);
    }
  };

  /// Abre o overlay pós-jogo. Tenta buscar partida real; usa preview mock se falhar.
  const showPostGame = async () => {
    if (postGameLoading.value) return;
    postGameLoading.value = true;
    try {
      await invoke('trigger_post_game_analysis');
    } catch (e: any) {
      console.warn('[PostGame] API falhou, abrindo preview:', e);
      // Preview com dados realistas para ver o layout
      const mockReport = {
        champion: 'Jinx',
        role: 'ADC',
        win: false,
        duration_min: 32.5,
        metrics: [
          { label: 'Mortes',         player_value: 9,    benchmark_value: 3.5, unit: 'mortes',  grade: 'D', feedback: '9 mortes — recue quando o JG inimigo não está visível.' },
          { label: 'KDA',            player_value: 0.6,  benchmark_value: 3.0, unit: 'ratio',   grade: 'D', feedback: 'KDA baixo — priorize sobreviver. Vivo vale mais que morto com kills.' },
          { label: 'CS por minuto',  player_value: 6.1,  benchmark_value: 8.2, unit: 'CS/min',  grade: 'C', feedback: 'Farm 2.1 CS/min abaixo do ideal — fique na rota nos primeiros 15min.' },
          { label: 'Visão',          player_value: 0.58, benchmark_value: 0.62, unit: 'vis/min', grade: 'B', feedback: 'Visão razoável. Control Ward antes dos objetivos sempre.' },
        ],
        overall_grade: 'D',
        priority_tip: 'Mortes: 9 mortes — recue quando o JG inimigo não está visível no mapa.',
      };
      postGameReport.value = mockReport;
      await openPostGameWindow(mockReport);
    } finally {
      postGameLoading.value = false;
    }
  };

  const openPostGameWindow = async (report: any) => {
    // localStorage é compartilhado entre todas as webviews do Tauri — sem race condition
    localStorage.setItem('spellcoach_postgame', JSON.stringify(report));

    try {
      const existing = await WebviewWindow.getByLabel("post-game");
      if (existing) await existing.close();
      await new Promise(r => setTimeout(r, 100));

      new WebviewWindow("post-game", {
        url: "index.html",
        title: "Spell Coach — Análise Pós-Jogo",
        width: 1024,
        height: 720,
        transparent: true,
        decorations: false,
        alwaysOnTop: true,
        center: true,
        focus: true,
      });
    } catch (e) {
      console.error('[PostGame] Erro ao abrir janela:', e);
    }
  };

  const openPreGameWindow = async (report: any) => {
    localStorage.setItem('spellcoach_pregame', JSON.stringify(report));
    try {
      const existing = await WebviewWindow.getByLabel("pre-game");
      if (existing) await existing.close();
      await new Promise(r => setTimeout(r, 100));
      new WebviewWindow("pre-game", {
        url: "index.html",
        title: "Spell Coach — Pré-Jogo",
        width: 1024,
        height: 720,
        transparent: true,
        decorations: false,
        alwaysOnTop: true,
        center: true,
        focus: true,
      });
    } catch (e) {
      console.error('[PreGame] Erro ao abrir janela:', e);
    }
  };

  /// Controla a exibição do cartão de dicas táticas (Flashcard) que desliza para a tela.
  /// PERFORMANCE: NÃO chamamos setFocus() aqui — roubar o foco do jogo causa lag/freeze
  /// no League of Legends em modo borderless. O flashcard é sempre passivo (passthrough de foco).
  const toggleFlashcard = async (forceState?: boolean, skipTips = false) => {
    try {
      let win = await WebviewWindow.getByLabel("flashcard");
      if (!win) {
        win = new WebviewWindow("flashcard", {
          url: "index.html",
          title: "Tactical Tip",
          width: 340,
          height: 160,
          transparent: true,
          decorations: false,
          alwaysOnTop: true,
          skipTaskbar: true,   // Não aparece na barra de tarefas — menos overhead do OS
          visible: false,
        });
        // Aguarda a janela ser criada APENAS na primeira vez
        await new Promise(r => setTimeout(r, 200));
      }

      const isVisible = await win.isVisible();
      const shouldShow = forceState !== undefined ? forceState : !isVisible;

      if (shouldShow) {
        // Busca dicas táticas APENAS se:
        // 1. Estamos no Champ Select (não in-game)
        // 2. skipTips = false (não é chamada interna da fila)
        // 3. Há um campeão ativo
        // 4. Ainda não buscamos dicas para este campeão (evita duplicar com o watcher)
        const isChampSelect = gameFlowState.value === 'CHAMP SELECT' || gameFlowState.value === 'CHAMPSELECT';
        const shouldFetch = activeChampion.value && !skipTips && isChampSelect
          && lastAutoTacticalTipChamp.value !== activeChampion.value;
        if (shouldFetch) {
          lastAutoTacticalTipChamp.value = activeChampion.value;
          fetchAndShowTacticalTips();
        }
        await win.emit("reset-flashcard");
        await win.show();
        // NÃO chamamos setFocus() — isso pausaria o jogo em fullscreen/borderless!
      } else if (isVisible) {
        // Não para a voz aqui — o ciclo de vida da voz é gerenciado pelo sistema de fila.
        await win.emit("close-flashcard");
        await new Promise(r => setTimeout(r, 600));
        await win.hide();
      }
    } catch (e) {
      console.error("[Toggle] Erro no toggleFlashcard:", e);
    }
  };

  // ---------------------------------------------------------------------------
  // 4. CHAMADAS DE IPC BACKEND (Tauri Invoke & Event Emits)
  // ---------------------------------------------------------------------------

  /// Consulta o backend Rust para obter as runas completas e exibe o overlay na tela.
  const fetchAndShowRuneOverlay = async () => {
    if (!activeChampion.value) return;
    
    // Apenas abre o overlay automaticamente se estivermos na fase de Seleção de Campeões
    const isChampSelect = gameFlowState.value === 'CHAMP SELECT' || gameFlowState.value === 'CHAMPSELECT';
    if (!isChampSelect) {
      console.log('[App.vue] Ignorando abertura do Rune Overlay pois não estamos em Champ Select. Estado atual:', gameFlowState.value);
      return;
    }

    try {
      console.log('[App.vue] Carregando runas de:', activeChampion.value, 'para a rota:', playerRole.value);
      
      // Invocação assíncrona do comando Tauri mapeado no Rust
      const data: any = await invoke("get_rune_overlay_data_command", { 
        champId: activeChampion.value,
        role: playerRole.value
      });
      if (data) {
        runeOverlayData.value = data;
        // Notifica as outras janelas/webviews sobre os novos dados de runa coletados
        await emit("update-rune-overlay-content", data);
        await toggleRuneOverlay(true);
      }
    } catch (e) {
      console.error("Erro ao buscar dados de runas:", e);
    }
  };

  /// Consulta o backend para dicas táticas do campeão (Groq ou fallback local).
  /// Funciona tanto no Champ Select quanto In-Game (quando Groq está ativo).
  const fetchAndShowTacticalTips = async () => {
    const champion = activeChampion.value;
    if (!champion || isFetchingTips.value) return;
    isFetchingTips.value = true;

    // Captura o estado ANTES do await — usado para detectar transições de fase
    const stateBeforeFetch = gameFlowState.value;

    try {
      const tips: any = await invoke("get_tactical_tips_command", {
        champId: champion,
        opponentKey: laneOpponentKey.value ?? null,
      });

      // Atualiza o estado de exaustão do Groq com base na resposta do backend.
      // groq_exhausted = true  → Groq falhou → libera procedural automaticamente
      // groq_exhausted = false → Groq funcionou → procedural bloqueado
      if (tips) {
        groqExhausted.value = !!tips.groq_exhausted;
      }

      // Guard pós-await: cancela se a sessão de CS foi encerrada enquanto aguardávamos o Groq
      const wasChampSelect = stateBeforeFetch === 'CHAMP SELECT' || stateBeforeFetch === 'CHAMPSELECT';
      const isNowInGame = gameFlowState.value === 'GAME' || gameFlowState.value === 'INGAME';
      if (wasChampSelect && !champSelectSessionActive && !isNowInGame) return; // dodge / voltou ao lobby
      if (wasChampSelect && isNowInGame && !groqEnabled.value) return; // CS→jogo sem Groq

      if (tips) {
        // Enfileira matchup como dica sequencial — aguarda o mutex antes de enfileirar itens
        if (tips.matchup_back) {
          await queueAndPlayTip({
            title: `${champion} — Matchup`,
            frontText: tips.matchup_front || "Dica de Rota",
            backText: tips.matchup_back,
            rarity: "epic"
          });
        }
        // Itens só são enfileirados se o matchup já foi (não emitidos ao mesmo tempo)
        if (tips.item_back) {
          await queueAndPlayTip({
            title: `${champion} — Itens`,
            frontText: "Compras Recomendadas",
            backText: tips.item_back,
            rarity: "rare"
          });
        }
      }
    } catch (e) {
      // Erro de rede ou Tauri IPC: libera procedural
      groqExhausted.value = true;
      console.error("Erro ao carregar dicas táticas:", e);
    } finally {
      isFetchingTips.value = false;
    }
  };


  // ---------------------------------------------------------------------------
  // 5. OBSERVADORES REATIVOS (Watchers)
  // ---------------------------------------------------------------------------
  // Para o estudante: Watchers escutam mudanças em propriedades reativas e disparam 
  // ações colaterais no sistema de forma declarativa e limpa.

  // Observa mudanças no campeão ativo
  watch(activeChampion, async (newChamp) => {
    if (windowLabel.value !== 'main') return;
    if (newChamp) {
      console.log('[App.vue] Novo campeão ativo selecionado:', newChamp);
      await fetchAndShowRuneOverlay();

      // Dicas táticas apenas dentro de uma sessão ativa de Champ Select (flag de transição real)
      if (champSelectSessionActive && lastAutoTacticalTipChamp.value !== newChamp) {
        lastAutoTacticalTipChamp.value = newChamp;
        await fetchAndShowTacticalTips();
      }
    } else {
      lastAutoTacticalTipChamp.value = null;
      lastFetchedTipsCombo = null; // reseta o guard de combo ao sair da sessão
      // Sem campeão ativo: para a voz, limpa a fila e fecha os widgets
      stopVoice();
      tipQueue.value = [];
      currentActiveTip.value = null;
      if (flashcardTimeout) {
        clearTimeout(flashcardTimeout);
        flashcardTimeout = null;
      }
      await toggleRuneOverlay(false);
      await toggleFlashcard(false);
    }
  });

  // Quando o oponente de rota travar depois que as dicas já foram buscadas sem ele,
  // re-busca com o oponente real para corrigir a dica (evita alucinações do Groq).
  let lastFetchedTipsCombo: string | null = null;

  watch(laneOpponentKey, async (newKey, oldKey) => {
    if (windowLabel.value !== 'main') return;
    if (newKey !== null && oldKey === null && activeChampion.value) {
      if (champSelectSessionActive) {
        // Guard: se já buscamos dicas para este par campeão+oponente, ignora o re-trigger
        // (laneOpponentKey pode oscilar null↔valor entre eventos LCU parciais)
        const combo = `${activeChampion.value}:${newKey}`;
        if (combo === lastFetchedTipsCombo) {
          console.log('[App.vue] Combo já buscado:', combo, '— ignorando re-trigger do watcher.');
          return;
        }
        lastFetchedTipsCombo = combo;
        console.log('[App.vue] Oponente de rota travou:', newKey, '— descartando fila antiga e re-buscando com oponente real.');
        // Descarta dicas antigas (geradas sem oponente real) para não acumular flashcards repetidos.
        tipQueue.value = [];
        if (isProcessingTip) {
          stopVoice();
          triggerDismiss();
          tipSessionId++;
          isProcessingTip = false;
          currentActiveTip.value = null;
          await toggleFlashcard(false);
        }
        lastAutoTacticalTipChamp.value = null;
        await fetchAndShowTacticalTips();
      }
    }
  });

  // Quando o Kokoro fica pronto durante o Champ Select, re-fala as dicas que foram
  // ignoradas enquanto o motor ainda estava carregando (timing issue na inicialização).
  watch(kokoroStatus, async (newStatus, oldStatus) => {
    if (windowLabel.value !== 'main') return;
    if (newStatus !== 'ready' || oldStatus === 'ready') return;
    const isChampSelect = gameFlowState.value === 'CHAMP SELECT' || gameFlowState.value === 'CHAMPSELECT';
    if (!isChampSelect || !activeChampion.value || !champSelectSessionActive) return;
    console.log('[useSpellCoach] Kokoro ficou pronto durante o Champ Select — re-buscando dicas táticas.');
    lastAutoTacticalTipChamp.value = null; // reseta guard para permitir re-fetch
    await fetchAndShowTacticalTips();
  });

  // Observa mudanças no estado do jogo (IDLE -> CHAMP SELECT -> GAME)
  let groqInGameInterval: ReturnType<typeof setInterval> | null = null;
  let groqGameStartTimeout: ReturnType<typeof setTimeout> | null = null;

  // Flag de sessão: só fica true após uma TRANSIÇÃO real para Champ Select.
  // Watchers de dicas verificam este flag em vez de ler gameFlowState diretamente,
  // evitando re-trigger por eventos LCU velhos ou HMR re-mount.
  let champSelectSessionActive = false;

  const clearInGameTimers = () => {
    if (groqInGameInterval) { clearInterval(groqInGameInterval); groqInGameInterval = null; }
    if (groqGameStartTimeout) { clearTimeout(groqGameStartTimeout); groqGameStartTimeout = null; }
  };

  watch(gameFlowState, async (newState, oldState) => {
    if (windowLabel.value !== 'main') return;
    try {
      const isGameActive    = newState === 'GAME' || newState === 'INGAME';
      const wasGameActive   = oldState === 'GAME' || oldState === 'INGAME';
      const isNowCS         = newState === 'CHAMP SELECT' || newState === 'CHAMPSELECT';
      const wasCS           = oldState === 'CHAMP SELECT' || oldState === 'CHAMPSELECT';

      // Entrada real no Champ Select → abre a sessão e reseta todos os guards de fetch
      if (isNowCS && !wasCS) {
        champSelectSessionActive = true;
        lastFetchedTipsCombo = null;
        lastAutoTacticalTipChamp.value = null;
        console.log('[App.vue] Sessão de Champ Select iniciada.');
      }

      // Saída do Champ Select (para jogo ou lobby) → fecha a sessão
      if (!isNowCS && wasCS) {
        champSelectSessionActive = false;
        console.log('[App.vue] Sessão de Champ Select encerrada.');
      }

      // Limpa UI do champ select SOMENTE na transição de entrada no jogo (não em blips de reconexão)
      if (isGameActive && !wasGameActive) {
        stopVoice();
        tipQueue.value = [];
        await toggleRuneOverlay(false);
        await toggleFlashcard(false);

        // Se Groq estiver ativo, dispara dicas periódicas via Groq in-game (a cada 60s)
        // substituindo completamente as dicas procedurais do Rust bridge.
        if (groqEnabled.value) {
          console.log('[App.vue] Groq ativo — iniciando ciclo de dicas in-game via Groq (60s).');
          // Primeira dica logo ao entrar no jogo (após 10s para estabilizar)
          groqGameStartTimeout = setTimeout(() => {
            groqGameStartTimeout = null;
            const stillInGame = gameFlowState.value === 'GAME' || gameFlowState.value === 'INGAME';
            if (activeChampion.value && stillInGame) {
              lastAutoTacticalTipChamp.value = null;
              fetchAndShowTacticalTips();
            }
          }, 10000);
          // Intervalo recorrente a cada 60s
          groqInGameInterval = setInterval(() => {
            const stillInGame = gameFlowState.value === 'GAME' || gameFlowState.value === 'INGAME';
            if (activeChampion.value && stillInGame) {
              lastAutoTacticalTipChamp.value = null;
              fetchAndShowTacticalTips();
            }
          }, 60000);
        }
      }

      // Quando sai do jogo: cancela timers, limpa fila e fecha flashcards pendentes
      if (!isGameActive && wasGameActive) {
        clearInGameTimers();
        console.log('[App.vue] Saiu do jogo — timers Groq encerrados, limpando fila de dicas.');
        stopVoice();
        tipQueue.value = [];
        if (isProcessingTip) {
          triggerDismiss();
          tipSessionId++;
          isProcessingTip = false;
          currentActiveTip.value = null;
        }
        await toggleFlashcard(false);
      }

      // Controla a exibição automática da barra horizontal de builds (Build Bar)
      try {
        const buildWin = await WebviewWindow.getByLabel("build");
        if (buildWin) {
          if (isGameActive) {
            await buildWin.show();
          } else {
            await buildWin.hide();
          }
        }
      } catch (_) {}
    } catch (e) {
      console.error("Error handling game state change:", e);
    }
  });


  // ─── MUTEX de reentrância ──────────────────────────────────────────────────
  // isProcessingTip: flag booleana (não-reativa) para exclusão mútua síncrona.
  // tipSessionId: contador de geração — incrementado em cada interrupção de emergência.
  // Isso permite que corrotinas antigas detectem que foram supersedidas e saiam sem
  // liberar o mutex nem acionar o próximo item da fila.
  let isProcessingTip = false;
  let tipSessionId = 0;

  // ─── SISTEMA DE DISMISS BASEADO EM DADOS ───────────────────────────────────
  // Em vez de um timer fixo (7s), cada dica fecha quando a condição de jogo é atingida.
  // _dismissResolve é a função que resolve a Promise retornada por waitForDismissSignal().
  let _dismissResolve: (() => void) | null = null;

  const waitForDismissSignal = (): Promise<void> => new Promise(resolve => {
    _dismissResolve = resolve;
  });

  const triggerDismiss = () => {
    if (_dismissResolve) {
      const fn = _dismissResolve;
      _dismissResolve = null;
      fn();
    }
  };

  const checkDismissCondition = (dismiss: DismissCondition, state: GameState): boolean => {
    switch (dismiss.type) {
      case 'fixed':            return false;
      case 'clear_step_gt':   return state.clear_step > (dismiss.target ?? 0);
      case 'level_gt':        return state.level > (dismiss.target ?? 999);
      case 'game_time_gte':   return state.game_time >= (dismiss.target ?? Infinity);
      case 'inventory_change': return state.inventory.length > 0;
      case 'skill_leveled':
        if (!dismiss.snapshot) return false;
        return state.abilities.q > dismiss.snapshot.q ||
               state.abilities.w > dismiss.snapshot.w ||
               state.abilities.e > dismiss.snapshot.e ||
               state.abilities.r > dismiss.snapshot.r;
      case 'fallback': return false; // gerenciado por setTimeout interno
      default:         return false;
    }
  };

  const showNextTip = async () => {
    // Guard de reentrância — rejeita chamada se já estamos processando
    if (isProcessingTip) return;

    // Marca mutex imediatamente (antes de qualquer await) para fechar a janela de corrida
    // onde dois eventos chegam antes da primeira suspensão da corrotina anterior.
    isProcessingTip = true;

    if (flashcardTimeout) {
      clearTimeout(flashcardTimeout);
      flashcardTimeout = null;
    }

    if (tipQueue.value.length === 0) {
      await toggleFlashcard(false);
      isProcessingTip = false;
      return;
    }

    // Descarta dicas velhas (enfileiradas há mais de 25s) — informação já é obsoleta
    const now = Date.now();
    tipQueue.value = tipQueue.value.filter(t => (now - t.timestamp) < 25000);

    if (tipQueue.value.length === 0) {
      await toggleFlashcard(false);
      isProcessingTip = false;
      return;
    }

    // Captura o ID de sessão desta corrotina — se uma emergência incrementar tipSessionId
    // enquanto aguardamos speak(), saberemos que fomos supersedidos e não devemos
    // liberar o mutex nem chamar showNextTip() ao final.
    const mySessionId = tipSessionId;

    // Pega a dica de maior prioridade no início da fila
    const nextTip = tipQueue.value.shift()!;
    currentActiveTip.value = nextTip;

    // Grava o timestamp da dica atual para verificar identidade após awaits longos
    const myTipTimestamp = nextTip.timestamp;

    console.log('[useSpellCoach] Executando próxima dica da fila de prioridade:', nextTip.title);

    // Customiza a dica dinamicamente com o Nickname do jogador
    const nick = getSummonerNickname(summonerName.value);
    const customizedBackText = customizeTextWithNickname(nextTip.backText, nick, nextTip.title);

    // Atualiza os dados locais do flashcard
    flashcardData.value = {
      title: nextTip.title,
      frontText: nextTip.frontText,
      backText: customizedBackText,
      rarity: nextTip.rarity as any
    };

    // Sincroniza e envia para todas as janelas do Tauri com o texto customizado
    await emit("update-flashcard-content-queued", {
      ...nextTip,
      backText: customizedBackText
    });

    // ── Verifica se voz está disponível ────────────────────────────────────
    // voiceActive=true  → card aparece junto com o speak; fecha quando o áudio terminar.
    // voiceActive=false → card aparece imediatamente; fecha por timer (comportamento padrão).
    const voiceActive = voiceEnabled.value && kokoroStatus.value === 'ready';
    const dismiss = nextTip.dismiss;

    if (voiceActive) {
      // ── MODO VOZ ATIVA ────────────────────────────────────────────────────
      // speak() invoca o Rust de forma bloqueante — a Promise só resolve quando o
      // áudio termina completamente. Abrimos o card em paralelo ao início do speak.
      await toggleFlashcard(true, true);

      if (dismiss && dismiss.type !== 'fallback') {
        // Condição de dados (jungle clear, level-up, etc.):
        // O áudio roda uma vez; o card permanece visível até a condição ser atingida.
        await speak(customizedBackText);       // aguarda o áudio terminar
        await waitForDismissSignal();          // aguarda condição de jogo
        stopVoice();
      } else if (dismiss?.type === 'fallback') {
        // Fallback: fecha quando o áudio terminar; timer de segurança como backup.
        const maxMs = dismiss.max_ms ?? 30000;
        const safetyTimer = setTimeout(() => triggerDismiss(), maxMs);
        await Promise.race([
          speak(customizedBackText),           // fecha ao terminar o áudio
          new Promise<void>(r => setTimeout(r, maxMs))  // ou no timeout máximo
        ]);
        clearTimeout(safetyTimer);
        triggerDismiss(); // resolve qualquer waitForDismissSignal pendente
        stopVoice();
      } else {
        // Sem condição de dismiss: fecha quando o áudio terminar, mínimo 3s de exibição.
        const minDisplay = new Promise<void>(r => setTimeout(r, 3000));
        await Promise.all([speak(customizedBackText), minDisplay]);
        stopVoice();
      }
    } else {
      // ── MODO SEM VOZ: card aparece imediatamente, fecha por timer ─────────
      await toggleFlashcard(true, true);

      if (dismiss && dismiss.type !== 'fallback') {
        // Condição de dados sem voz — aguarda dismiss signal normalmente
        await waitForDismissSignal();
      } else if (dismiss?.type === 'fallback') {
        // Fallback sem voz: usa timer mais curto (7s) — sem áudio para calibrar duração
        const maxMs = Math.min(dismiss.max_ms ?? 30000, 7000);
        const fallbackTimer = setTimeout(() => triggerDismiss(), maxMs);
        await waitForDismissSignal();
        clearTimeout(fallbackTimer);
      } else {
        // Sem dismiss: exibe por 7s
        await new Promise<void>(r => setTimeout(r, 7000));
      }
    }

    // Verifica se fomos supersedidos por uma emergência ou substituição de jungle clear.
    // Se sim, saímos sem tocar no mutex — a nova cadeia assume o controle.
    if (tipSessionId !== mySessionId) {
      console.log('[useSpellCoach] Dica', nextTip.title, 'supersedida — saindo sem liberar mutex.');
      return;
    }

    // Verifica identidade antes de fechar o card.
    const iStillOwn = currentActiveTip.value?.timestamp === myTipTimestamp;
    if (iStillOwn) {
      await toggleFlashcard(false);
      currentActiveTip.value = null;
    } else {
      console.log('[useSpellCoach] Dica', nextTip.title, 'foi substituída por outra — não fechando o card.');
    }

    // Cooldown mínimo de 1.5s entre dicas para evitar rajadas visuais
    await new Promise(r => setTimeout(r, 1500));

    // Libera o mutex
    isProcessingTip = false;

    // Processa próxima dica da fila (se houver)
    if (tipQueue.value.length > 0) {
      showNextTip();
    }
  };

  const queueAndPlayTip = async (tip: Omit<QueuedTip, 'priority' | 'timestamp'>) => {
    const priority = getRarityPriority(tip.rarity, tip.title);
    const newQueuedTip: QueuedTip = {
      ...tip,
      priority,
      timestamp: Date.now()
    };

    console.log('[useSpellCoach] Inserindo dica na fila de prioridades:', newQueuedTip.title, '| prioridade:', priority);

    if (!isProcessingTip) {
      // Mutex livre — reproduz imediatamente
      tipQueue.value.push(newQueuedTip);
      showNextTip(); // não-await: deixa rodar em background para não bloquear o listener
    } else {
      const isNewJungleClear    = newQueuedTip.tipCategory === 'jungle_clear';
      const isCurrentJungleClear = currentActiveTip.value?.tipCategory === 'jungle_clear';

      // Cada novo step de jungle clear substitui imediatamente o step anterior —
      // independente de prioridade, pois são dicas sequenciais do mesmo sistema.
      if (isNewJungleClear && isCurrentJungleClear) {
        console.log('[useSpellCoach] Novo step de jungle clear — substituindo step atual:', currentActiveTip.value?.title, '->', newQueuedTip.title);
        stopVoice();
        triggerDismiss(); // libera waitForDismissSignal() da corrotina atual
        tipQueue.value.unshift(newQueuedTip);
        tipSessionId++;   // invalida corrotina do step anterior
        isProcessingTip = false;
        currentActiveTip.value = null;
        showNextTip();
        return;
      }

      // Interrompe para alertas de emergência (prioridade >= 4: gank/perigo).
      const isEmergency = newQueuedTip.priority >= 4 &&
        (currentActiveTip.value ? newQueuedTip.priority > currentActiveTip.value.priority : true);

      if (isEmergency) {
        console.log('[useSpellCoach] Emergência — interrompendo dica atual:', currentActiveTip.value?.title, '->', newQueuedTip.title);

        stopVoice();
        triggerDismiss(); // libera o waitForDismissSignal() da corrotina atual

        // Recoloca a dica interrompida na fila para tocar depois do alerta
        if (currentActiveTip.value) {
          tipQueue.value.unshift({ ...currentActiveTip.value, timestamp: Date.now() - 1000 });
        }
        tipQueue.value.unshift(newQueuedTip);
        tipQueue.value.sort((a, b) => b.priority - a.priority || a.timestamp - b.timestamp);

        // Invalida a corrotina antiga — ela detectará tipSessionId !== mySessionId e sairá.
        tipSessionId++;
        isProcessingTip = false;
        currentActiveTip.value = null;
        showNextTip();
      } else {
        // Dica normal: enfileira por prioridade sem interromper a voz atual.
        // Limita a fila a 3 dicas pendentes — descarta as de menor prioridade.
        tipQueue.value.push(newQueuedTip);
        tipQueue.value.sort((a, b) => b.priority - a.priority || a.timestamp - b.timestamp);
        if (tipQueue.value.length > 3) {
          const descartadas = tipQueue.value.splice(3);
          console.log('[useSpellCoach] Fila cheia — descartando dicas de baixa prioridade:', descartadas.map(d => d.title));
        }
      }
    }
  };

  // ---------------------------------------------------------------------------
  // 6. CICLOS DE VIDA E LISTENERS DE EVENTOS TAURI (Tauri Event Bridge)
  // ---------------------------------------------------------------------------
  // Para o estudante:
  // onMounted() roda quando a tela é carregada pela primeira vez. 
  // Aqui ligamos os Listeners de eventos que o backend Rust emite via websockets do Tauri.

  let unlistenUpdate: () => void;
  let unlistenClose: any;
  let unlistenReset: any;
  let flashcardTimeout: any = null;

  onMounted(async () => {
    // Flashcard é display-only (sem interação do usuário) — passa todos os cliques ao jogo.
    // build, ward-map e main: NÃO usam setIgnoreCursorEvents — o Tauri já passa clicks
    // por pixels transparentes (alpha=0) nativamente. Usar setIgnoreCursorEvents(true) nas
    // janelas com botões de fechar bloquearia os próprios botões (catch-22).
    if (windowLabel.value === 'flashcard') {
      try { await appWindow.setIgnoreCursorEvents(true); } catch (_) {}
    }

    // Escuta atualizações de estado globais emitidas pelo ciclo LCU do Rust (bridge.rs)
    unlistenUpdate = await listen("lcu-update", (event: any) => {
      const { status, summoner, state } = event.payload;
      lcuStatus.value = status;
      isLive.value = status === "Connected";
      if (isLive.value) {
        summonerName.value = summoner.gameName ? `${summoner.gameName}#${summoner.tagLine}` : (summoner.displayName || "Invocador");
        if (typeof state === 'string') gameFlowState.value = state.toUpperCase();
        else if (state?.ChampSelect) {
          gameFlowState.value = "CHAMP SELECT";
          const rawRole = state.ChampSelect.role || '';
          if (rawRole) {
            playerRole.value = getNormalizedRole(rawRole);
          }
          // Extrai adversário real de rota para o Groq não usar o pior matchup estatístico
          const theirTeam: any[] = state.ChampSelect.their_team || [];
          const laneOpp = theirTeam.find((m: any) =>
            m.assigned_position?.toLowerCase() === rawRole.toLowerCase() && m.champion_id > 0
          );
          laneOpponentKey.value = laneOpp ? laneOpp.champion_id : null;
        } else {
          gameFlowState.value = "GAME";
          laneOpponentKey.value = null;
        }

        // Coleta dinamicamente o campeão em Champ Select ou Game ativo
        const isChampSelect = gameFlowState.value === 'CHAMP SELECT' || gameFlowState.value === 'CHAMPSELECT';
        if (isChampSelect) {
          // Só atualiza se temos um valor — eventos parciais sem championName não limpam o estado
          if (event.payload.championName) {
            activeChampion.value = event.payload.championName;
          }
        } else if (gameFlowState.value === "INGAME" && event.payload.gameData) {
          const gameData = event.payload.gameData;
          const activeSumm: string = gameData.activePlayer?.summonerName || '';
          const activeBase = activeSumm.split('#')[0]?.toLowerCase() || '';
          // activePlayer.championName ausente em patches recentes — busca em allPlayers
          let champ: string = gameData.activePlayer?.championName || '';
          if (!champ && activeSumm) {
            const match = (gameData.allPlayers as any[] | undefined)?.find((p: any) => {
              const pName: string = p.summonerName || '';
              const pBase = pName.split('#')[0]?.toLowerCase() || '';
              return pName === activeSumm || (activeBase && pBase === activeBase);
            });
            champ = match?.championName || '';
          }
          if (champ) activeChampion.value = champ;
        } else {
          activeChampion.value = null;
        }
      } else {
        // Reset de estado se o jogo fechar ou desconectar
        summonerName.value = "---";
        gameFlowState.value = "IDLE";
        activeChampion.value = null;
      }
    });

    // Se esta Webview específica for a do widget de Dica (flashcard), liga seus hooks de animação
    if (windowLabel.value === 'flashcard') {
      unlistenClose = await listen("close-flashcard", () => {
        isExitingFlashcard.value = true;
      });
      unlistenReset = await listen("reset-flashcard", () => {
        isExitingFlashcard.value = false;
        flashcardKey.value++; 
      });

      await listen("respond-flashcard-content", (event: any) => {
        console.log('[App.vue] Janela flashcard inicializada com dados:', event.payload);
        flashcardData.value = event.payload;
      });
      
      // Solicita os dados carregados na janela principal (Tauri IPC de sincronização interna)
      console.log('[App.vue] Janela flashcard montada. Solicitando conteúdo da dica...');
      emit("request-flashcard-content");
    }

    // Gerencia a ponte interna de sincronização de dados entre as janelas do Tauri
    if (windowLabel.value === 'main') {
      await listen("request-flashcard-content", () => {
        console.log('[App.vue] Janela principal enviando conteúdo do flashcard atual:', flashcardData.value);
        emit("respond-flashcard-content", flashcardData.value);
      });

      await listen("request-rune-overlay-content", () => {
        console.log('[App.vue] Janela principal enviando conteúdo de runas atual:', runeOverlayData.value);
        emit("update-rune-overlay-content", runeOverlayData.value);
      });

      await listen("hide-rune-overlay", async () => {
        console.log('[App.vue] Evento hide-rune-overlay recebido. Ocultando overlay...');
        await toggleRuneOverlay(false);
      });
    }

    // Ward Map: recebe evento do Rust e exibe/atualiza a janela
    let wardMapCloseTimer: ReturnType<typeof setTimeout> | null = null;

    if (windowLabel.value === 'main') {
      await listen("update-ward-map", async (event: any) => {
        const d = event.payload;
        if (!d || !d.wards?.length) return; // ignora eventos sem wards

        // Cancela o timer de fechamento anterior — evita que timers acumulados
        // fechem o mapa mais cedo do que o previsto pelo evento mais recente.
        if (wardMapCloseTimer) {
          clearTimeout(wardMapCloseTimer);
          wardMapCloseTimer = null;
        }

        wardMapData.value = {
          champion:        d.champion         || "",
          role:            d.role             || "MID",
          phase:           d.phase            || "early",
          teamSide:        d.team_side        || "blue",
          wards:           d.wards            || [],
          gameTime:        d.game_time        || 0,
          objective:       d.objective        || "",
          objectiveEmoji:  d.objective_emoji  || "",
          secondsToSpawn:  d.seconds_to_spawn || 0,
        };
        // Persiste no localStorage para que a janela ward-map recupere os dados
        // mesmo que o evento Tauri chegue antes do listener estar pronto.
        try { localStorage.setItem('spellcoach_wardmap', JSON.stringify(wardMapData.value)); } catch (_) {}
        // Abre a janela (cria se necessário, aguarda carregamento)
        await toggleWardMap(true);
        // Emite dados APÓS a janela estar aberta e carregada
        try {
          await emit("ward-map-data-updated", wardMapData.value);
        } catch (_) {}
        // Objetivo: 15s (urgente), genérico: 12s — tempo suficiente para o jogador ler
        const displayMs = d.objective ? 15000 : (d.display_secs ? d.display_secs * 1000 : 12000);
        wardMapCloseTimer = setTimeout(() => {
          wardMapCloseTimer = null;
          toggleWardMap(false);
        }, displayMs);
        console.log('[WardMap] Recebido:', d.champion, d.role, d.wards?.length, 'pontos, exibindo por', displayMs / 1000, 's');
      });
    }

    // Ward Map: atualiza dados quando a janela é aberta
    if (windowLabel.value === 'ward-map') {
      // Recupera dados do localStorage como estado inicial (garante que wards aparecem
      // mesmo que o evento Tauri tenha chegado antes do listener estar pronto).
      try {
        const stored = localStorage.getItem('spellcoach_wardmap');
        if (stored) {
          const parsed = JSON.parse(stored);
          if (parsed?.wards?.length) wardMapData.value = parsed;
        }
      } catch (_) {}

      await listen("ward-map-data-updated", (event: any) => {
        if (event.payload?.wards?.length) {
          wardMapData.value = event.payload;
        }
      });
    }

    // Análise pré-jogo — abre durante o loading screen com matchups 1v1
    if (windowLabel.value === 'main') {
      await listen("pre-game-analysis", async (event: any) => {
        if (!event.payload) return;
        preGameReport.value = event.payload;
        await openPreGameWindow(event.payload);
        console.log('[PreGame] Análise recebida:', event.payload?.player_champion, event.payload?.matchups?.length, 'matchups');
      });
    }

    // Relatório pós-jogo — abre janela centralizada com análise da partida
    if (windowLabel.value === 'main') {
      await listen("post-game-report", async (event: any) => {
        if (!event.payload) return;
        postGameReport.value = event.payload;
        await openPostGameWindow(event.payload);
        console.log('[PostGame] Relatório recebido:', event.payload?.champion, event.payload?.overall_grade);
      });
    }

    // Janela post-game: lê do localStorage — disponível imediatamente ao montar
    if (windowLabel.value === 'post-game') {
      const stored = localStorage.getItem('spellcoach_postgame');
      if (stored) {
        try {
          postGameReport.value = JSON.parse(stored);
        } catch (e) {
          console.error('[PostGame] Erro ao parsear relatório:', e);
        }
      }
    }

    // Limpa fila de tips ao iniciar partida — garante que tips de ban/pick não toquem
    // antes do clear da jungle (ou de qualquer tip de início de jogo).
    if (windowLabel.value === 'main') {
      await listen("game-started", async () => {
        console.log('[App.vue] Partida iniciada — limpando fila de tips do champ select.');
        stopVoice();
        tipQueue.value = [];
        currentActiveTip.value = null;
      });
    }

    // Notificação de atualização disponível — aparece 5s após o app iniciar (se houver update).
    if (windowLabel.value === 'main') {
      await listen("update-available", async (event: any) => {
        const { version, notes } = event.payload ?? {};
        if (!version) return;
        console.log(`[Updater] Versão ${version} disponível.`);
        
        // Passa os dados para a janela de atualização via localStorage
        localStorage.setItem('spellcoach_update_data', JSON.stringify({ version, notes }));
        
        // Exibe a janela customizada de atualização (Nexus Explosion)
        const updaterWindow = await WebviewWindow.getByLabel('updater');
        if (updaterWindow) {
          await updaterWindow.show();
          await updaterWindow.setFocus();
        }
      });
    }

    // Apenas a janela principal (main) intercepta o evento bruto de dica do Rust e enfileira
    if (windowLabel.value === 'main') {
      await listen("update-flashcard-content", async (event: any) => {
        // EXCLUSIVIDADE: se o Groq está configurado E ainda não foi esgotado, bloqueia procedural.
        // Quando o Groq esgota tokens/rate-limit, groqExhausted vira true e as dicas
        // procedurais do Rust bridge são liberadas automaticamente como fallback.
        const isInGame = gameFlowState.value === 'GAME' || gameFlowState.value === 'INGAME';
        const groqBlockingProcedural = groqEnabled.value && !groqExhausted.value;
        if (groqBlockingProcedural && isInGame) return;
        if (event.payload) {
          await queueAndPlayTip({
            title: event.payload.title || "Dica do Coach",
            frontText: event.payload.frontText || "Info",
            backText: event.payload.backText || "",
            rarity: event.payload.rarity || "epic",
            dismiss: event.payload.dismiss as DismissCondition | undefined,
            tipCategory: event.payload.tipCategory as string | undefined
          });
        }
      });

      // Escuta o estado do jogo a cada tick (2s) — verifica se a dica ativa deve fechar.
      // Este é o coração do sistema de dismiss baseado em dados.
      await listen("game-state-update", (event: any) => {
        if (!currentActiveTip.value?.dismiss) return;
        const state = event.payload as GameState;
        if (checkDismissCondition(currentActiveTip.value.dismiss, state)) {
          console.log('[useSpellCoach] Condição de dismiss atingida para:', currentActiveTip.value.title, '| tipo:', currentActiveTip.value.dismiss.type);
          stopVoice();
          triggerDismiss();
        }
      });
    }

    // Todas as janelas escutam o evento sincronizado da dica ativada pela fila de prioridades
    await listen("update-flashcard-content-queued", (event: any) => {
      flashcardData.value = event.payload;
    });

    // Inicia a verificação de sanidade do banco e sincronização silenciosa em segundo plano (Vercel Core Sync)
    if (windowLabel.value === 'main') {
      console.log('[App.vue] Janela principal pronta, aguardando inicialização do banco...');
      
      (async () => {
        let dbReady = false;
        for (let i = 0; i < 100; i++) {
          try {
            dbReady = await invoke<boolean>('is_db_ready');
            if (dbReady) {
              console.log('[App.vue] Banco de dados pronto!');
              break;
            }
          } catch (e) {
            console.warn('[App.vue] Erro ao verificar estado do banco:', e);
          }
          await new Promise(r => setTimeout(r, 100));
        }

        if (dbReady) {
          console.log('[App.vue] Banco de dados pronto. Agendando sincronização silenciosa para daqui a 1 segundo...');
          setTimeout(() => {
            console.log('[App.vue] Iniciando sincronização em segundo plano via Vercel...');
            invoke('sync_vercel_command').catch(e => {
              console.error('[App.vue] Falha ao invocar sync_vercel_command:', e);
            });
          }, 1000);
        } else {
          console.error('[App.vue] Falha crítica: O banco de dados não ficou pronto a tempo.');
        }
      })();
    }

    // Carrega configuração do Groq para saber qual motor de dicas usar
    if (windowLabel.value === 'main') {
      try {
        const groqSettings: any = await invoke('get_groq_settings');
        groqEnabled.value = !!(groqSettings?.api_key?.trim());
        console.log('[App.vue] Motor de dicas:', groqEnabled.value ? 'Groq (IA Nuvem)' : 'Procedural (Rust Bridge)');
      } catch (e) {
        groqEnabled.value = false;
      }
    }
  });

  // Limpa os listeners assíncronos quando o componente é destruído para evitar vazamento de memória (Memory Leaks)!
  onUnmounted(() => {
    if (unlistenUpdate) unlistenUpdate();
    if (unlistenClose) unlistenClose();
    if (unlistenReset) unlistenReset();
  });

  // Expõe o estado reativo e os métodos para serem importados e utilizados nos templates HTML
  return {
    appWindow,
    windowLabel,
    settingsLoading,
    dataViewerLoading,
    isLive,
    summonerName,
    lcuStatus,
    gameFlowState,
    activeChampion,
    runeOverlayData,
    playerRole,
    flashcardKey,
    isExitingFlashcard,
    flashcardData,
    wardMapData,
    postGameReport,
    postGameLoading,
    preGameReport,
    openSettings,
    openDataViewer,
    toggleRuneOverlay,
    toggleWardMap,
    toggleFlashcard,
    showPostGame,
    fetchAndShowRuneOverlay,
    fetchAndShowTacticalTips,
  };
}
