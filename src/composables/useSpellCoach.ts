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
  const toggleWardMap = async (forceState?: boolean) => {
    try {
      let win = await WebviewWindow.getByLabel("ward-map");
      if (!win) {
        win = new WebviewWindow("ward-map", {
          url: "index.html",
          title: "Ward Map",
          width: 280,
          height: 360,
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
      const tips: any = await invoke("get_tactical_tips_command", { champId: champion });

      // Atualiza o estado de exaustão do Groq com base na resposta do backend.
      // groq_exhausted = true  → Groq falhou → libera procedural automaticamente
      // groq_exhausted = false → Groq funcionou → procedural bloqueado
      if (tips) {
        groqExhausted.value = !!tips.groq_exhausted;
      }

      // Guard pós-await: cancela se era Champ Select e o jogo já começou enquanto aguardávamos
      // (evita que dicas de champ select apareçam in-game quando Groq não está ativo)
      const wasChampSelect = stateBeforeFetch === 'CHAMP SELECT' || stateBeforeFetch === 'CHAMPSELECT';
      const isNowInGame = gameFlowState.value === 'GAME' || gameFlowState.value === 'INGAME';
      if (wasChampSelect && isNowInGame && !groqEnabled.value) return;

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

      // Dicas táticas do OpenRouter apenas no Champ Select — evita duplicar com dicas in-game do Rust
      const isChampSelect = gameFlowState.value === 'CHAMP SELECT' || gameFlowState.value === 'CHAMPSELECT';
      if (isChampSelect && lastAutoTacticalTipChamp.value !== newChamp) {
        lastAutoTacticalTipChamp.value = newChamp;
        await fetchAndShowTacticalTips();
      }
    } else {
      lastAutoTacticalTipChamp.value = null;
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

  // Observa mudanças no estado do jogo (IDLE -> CHAMP SELECT -> GAME)
  let groqInGameInterval: ReturnType<typeof setInterval> | null = null;

  watch(gameFlowState, async (newState, oldState) => {
    if (windowLabel.value !== 'main') return;
    try {
      const isGameActive = newState === 'GAME' || newState === 'INGAME';
      const wasGameActive = oldState === 'GAME' || oldState === 'INGAME';

      // Limpa UI do champ select SOMENTE na transição de entrada no jogo (não em blips de reconexão)
      if (isGameActive && !wasGameActive) {
        stopVoice();
        await toggleRuneOverlay(false);
        await toggleFlashcard(false);

        // Se Groq estiver ativo, dispara dicas periódicas via Groq in-game (a cada 60s)
        // substituindo completamente as dicas procedurais do Rust bridge.
        if (groqEnabled.value) {
          console.log('[App.vue] Groq ativo — iniciando ciclo de dicas in-game via Groq (60s).');
          // Primeira dica logo ao entrar no jogo (após 10s para estabilizar)
          setTimeout(() => {
            if (activeChampion.value) {
              lastAutoTacticalTipChamp.value = null; // reseta para permitir nova busca
              fetchAndShowTacticalTips();
            }
          }, 10000);
          // Intervalo recorrente a cada 60s
          groqInGameInterval = setInterval(() => {
            if (activeChampion.value) {
              lastAutoTacticalTipChamp.value = null;
              fetchAndShowTacticalTips();
            }
          }, 60000);
        }
      }

      // Quando sai do jogo: para o intervalo Groq
      if (!isGameActive && wasGameActive) {
        if (groqInGameInterval) {
          clearInterval(groqInGameInterval);
          groqInGameInterval = null;
          console.log('[App.vue] Saiu do jogo — intervalo Groq in-game encerrado.');
        }
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

    // Reproduz áudio do coach e exibe o widget
    await toggleFlashcard(true, true);

    const dismiss = nextTip.dismiss;

    if (dismiss && dismiss.type !== 'fallback') {
      // Dica com condição de dados: voz inicia mas não bloqueia — fecha quando situação mudar.
      // Para 'fixed': fecha apenas quando o próximo jungle clear step substituir esta dica.
      speak(customizedBackText);
      await waitForDismissSignal();
      stopVoice();
    } else if (dismiss?.type === 'fallback') {
      // Fallback com tempo máximo: voz roda + timeout de segurança em paralelo.
      const maxMs = dismiss.max_ms ?? 30000;
      const fallbackTimer = setTimeout(() => triggerDismiss(), maxMs);
      speak(customizedBackText);
      await waitForDismissSignal();
      clearTimeout(fallbackTimer);
      stopVoice();
    } else {
      // Sem condição de dismiss: comportamento legado (aguarda voz + 7s mínimo se sem voz).
      await speak(customizedBackText);
      if (!voiceEnabled.value || kokoroStatus.value !== 'ready') {
        await new Promise(r => setTimeout(r, 7000));
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
    // Janelas de display-only: passa cliques ao jogo (não captura mouse)
    const displayOnlyWindows = ['flashcard', 'build', 'ward-map', 'rune-overlay'];
    if (displayOnlyWindows.includes(windowLabel.value)) {
      try {
        await appWindow.setIgnoreCursorEvents(true);
      } catch (_) {}
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
          if (state.ChampSelect.role) {
            playerRole.value = getNormalizedRole(state.ChampSelect.role);
          }
        } else gameFlowState.value = "GAME";

        // Coleta dinamicamente o campeão em Champ Select ou Game ativo
        const isChampSelect = gameFlowState.value === 'CHAMP SELECT' || gameFlowState.value === 'CHAMPSELECT';
        if (isChampSelect) {
          if (event.payload.championName) {
            activeChampion.value = event.payload.championName;
          } else {
            activeChampion.value = null;
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
    if (windowLabel.value === 'main') {
      await listen("update-ward-map", async (event: any) => {
        const d = event.payload;
        if (!d) return;
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
        // Abre a janela (cria se necessário, aguarda carregamento)
        await toggleWardMap(true);
        // Emite dados APÓS a janela estar aberta e carregada
        try {
          await emit("ward-map-data-updated", wardMapData.value);
        } catch (_) {}
        setTimeout(() => toggleWardMap(false), 15000);
        console.log('[WardMap] Recebido:', d.champion, d.role, d.wards?.length, 'pontos');
      });
    }

    // Ward Map: atualiza dados quando a janela é aberta
    if (windowLabel.value === 'ward-map') {
      await listen("ward-map-data-updated", (event: any) => {
        if (event.payload) {
          wardMapData.value = event.payload;
        }
      });
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
        const { ask } = await import('@tauri-apps/plugin-dialog');
        const confirmed = await ask(
          `Spell Coach IA ${version} está disponível!\n\n${notes ? notes.slice(0, 300) : ''}\n\nDeseja instalar agora? O app vai reiniciar.`,
          { title: '🎮 Atualização Disponível', kind: 'info', okLabel: 'Instalar', cancelLabel: 'Depois' }
        );
        if (confirmed) {
          console.log('[Updater] Iniciando download e instalação...');
          await invoke('download_and_install_update').catch((e: any) =>
            console.error('[Updater] Erro ao instalar:', e)
          );
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
    openSettings,
    openDataViewer,
    toggleRuneOverlay,
    toggleWardMap,
    toggleFlashcard,
    fetchAndShowRuneOverlay,
    fetchAndShowTacticalTips,
  };
}
