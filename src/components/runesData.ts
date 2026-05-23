export interface RuneInfo {
  id: number;
  name: string;
  icon: string;
}

export interface TreeStructure {
  id: number;
  name: string;
  keystones: RuneInfo[];
  rows: RuneInfo[][];
}

export const RUNE_COLORS: Record<number, string> = {
  8000: '#C89B3C', // Precision (Gold)
  8100: '#DC4747', // Domination (Red)
  8200: '#4980F7', // Sorcery (Blue)
  8300: '#47AFDC', // Inspiration (Teal)
  8400: '#2FA84A'  // Resolve (Green)
};

// Season 16 — synced with CDragon perkstyles.json + perks.json
export const TREE_STRUCTURES: Record<number, TreeStructure> = {
  8000: {
    id: 8000,
    name: 'Precisão',
    keystones: [
      { id: 8005, name: 'Pressione o Ataque',  icon: 'Styles/Precision/PressTheAttack/PressTheAttack.png' },
      { id: 8008, name: 'Ritmo Fatal',          icon: 'Styles/Precision/LethalTempo/LethalTempoTemp.png' },
      { id: 8021, name: 'Agilidade nos Pés',    icon: 'Styles/Precision/FleetFootwork/FleetFootwork.png' },
      { id: 8010, name: 'Conquistador',          icon: 'Styles/Precision/Conqueror/Conqueror.png' }
    ],
    rows: [
      // Heroism: [9101, 9111, 8009]
      [
        { id: 9101, name: 'Absorver Vida',        icon: 'Styles/Precision/AbsorbLife/AbsorbLife.png' },
        { id: 9111, name: 'Triunfo',             icon: 'Styles/Precision/Triumph.png' },
        { id: 8009, name: 'Presença de Espírito',icon: 'Styles/Precision/PresenceOfMind/PresenceOfMind.png' }
      ],
      // Legend: [9104, 9105, 9103] — 9105 virou "Haste" no S15
      [
        { id: 9104, name: 'Lenda: Alacridade',  icon: 'Styles/Precision/LegendAlacrity/LegendAlacrity.png' },
        { id: 9105, name: 'Lenda: Rapidez',     icon: 'Styles/Precision/LegendHaste/LegendHaste.png' },
        { id: 9103, name: 'Lenda: Linhagem',    icon: 'Styles/Precision/LegendBloodline/LegendBloodline.png' }
      ],
      // Combat: [8014, 8017, 8299]
      [
        { id: 8014, name: 'Golpe de Misericórdia', icon: 'Styles/Precision/CoupDeGrace/CoupDeGrace.png' },
        { id: 8017, name: 'Dilacerar',              icon: 'Styles/Precision/CutDown/CutDown.png' },
        { id: 8299, name: 'Até a Morte',            icon: 'Styles/Sorcery/LastStand/LastStand.png' }
      ]
    ]
  },

  8100: {
    id: 8100,
    name: 'Dominação',
    // Predator (8124) removido no S15 — keystones agora têm 3 opções
    keystones: [
      { id: 8112, name: 'Eletrocutar',      icon: 'Styles/Domination/Electrocute/Electrocute.png' },
      { id: 8128, name: 'Colheita Sombria', icon: 'Styles/Domination/DarkHarvest/DarkHarvest.png' },
      { id: 9923, name: 'Chuva de Lâminas', icon: 'Styles/Domination/HailOfBlades/HailOfBlades.png' }
    ],
    rows: [
      // Malice: [8126, 8139, 8143] — igual
      [
        { id: 8126, name: 'Golpe Desleal',    icon: 'Styles/Domination/CheapShot/CheapShot.png' },
        { id: 8139, name: 'Gosto de Sangue',  icon: 'Styles/Domination/TasteOfBlood/GreenTerror_TasteOfBlood.png' },
        { id: 8143, name: 'Impacto Repentino',icon: 'Styles/Domination/SuddenImpact/SuddenImpact.png' }
      ],
      // Tracking: [8137, 8140, 8141] — NOVA ROW (era ZombieWard/GhostPoro/EyeballCollection)
      [
        { id: 8137, name: 'Sexto Sentido',       icon: 'Styles/Domination/SixthSense/SixthSense.png' },
        { id: 8140, name: 'Lembranças Sombrias', icon: 'Styles/Domination/GrislyMementos/GrislyMementos.png' },
        { id: 8141, name: 'Sentinela Profunda',  icon: 'Styles/Domination/DeepWard/DeepWard.png' }
      ],
      // Hunter: [8135, 8105, 8106] — Caçador Incansável (8134) removido
      [
        { id: 8135, name: 'Caçador de Tesouros', icon: 'Styles/Domination/TreasureHunter/TreasureHunter.png' },
        { id: 8105, name: 'Caçador Incansável',  icon: 'Styles/Domination/RelentlessHunter/RelentlessHunter.png' },
        { id: 8106, name: 'Caçadora Suprema',    icon: 'Styles/Domination/UltimateHunter/UltimateHunter.png' }
      ]
    ]
  },

  8200: {
    id: 8200,
    name: 'Feitiçaria',
    // Deathfire Touch (8992) adicionado como keystone no S16
    keystones: [
      { id: 8214, name: 'Invocação: Aery',           icon: 'Styles/Sorcery/SummonAery/SummonAery.png' },
      { id: 8229, name: 'Cometa Arcano',              icon: 'Styles/Sorcery/ArcaneComet/ArcaneComet.png' },
      { id: 8230, name: 'Arremetida do Conquistador', icon: 'Styles/Sorcery/PhaseRush/StormraidersSurgeRuneIcon2.png' },
      { id: 8992, name: 'Toque de Fogo Mortífero',   icon: 'Styles/Sorcery/DeathfireTouch/DEATHFIRE_TOUCH_KEYSTONE.png' }
    ],
    rows: [
      // Artifact: [8224, 8226, 8275] — 8224 virou "Axiom Arcanist" com novo ícone
      [
        { id: 8224, name: 'Arcanista Axiomático',   icon: 'Styles/Sorcery/NullifyingOrb/Axiom_Arcanist.png' },
        { id: 8226, name: 'Faixa de Fluxo de Mana', icon: 'Styles/Sorcery/ManaflowBand/ManaflowBand.png' },
        { id: 8275, name: 'Manto de Nimbus',         icon: 'Styles/Sorcery/NimbusCloak/6361.png' }
      ],
      // Excellence: [8210, 8234, 8233]
      [
        { id: 8210, name: 'Transcendência',  icon: 'Styles/Sorcery/Transcendence/Transcendence.png' },
        { id: 8234, name: 'Celeridade',      icon: 'Styles/Sorcery/Celerity/CelerityTemp.png' },
        { id: 8233, name: 'Foco Absoluto',   icon: 'Styles/Sorcery/AbsoluteFocus/AbsoluteFocus.png' }
      ],
      // Power: [8237, 8232, 8236] — 8232/8236 estavam com nomes e ícones trocados
      [
        { id: 8237, name: 'Chamuscar',               icon: 'Styles/Sorcery/Scorch/Scorch.png' },
        { id: 8232, name: 'Caminhar Sobre as Águas', icon: 'Styles/Sorcery/Waterwalking/Waterwalking.png' },
        { id: 8236, name: 'Tempestade Crescente',    icon: 'Styles/Sorcery/GatheringStorm/GatheringStorm.png' }
      ]
    ]
  },

  8400: {
    id: 8400,
    name: 'Determinação',
    keystones: [
      { id: 8437, name: 'Aperto Morto-Vivo', icon: 'Styles/Resolve/GraspOfTheUndying/GraspOfTheUndying.png' },
      { id: 8439, name: 'Pós-choque',         icon: 'Styles/Resolve/VeteranAftershock/VeteranAftershock.png' },
      { id: 8465, name: 'Guardião',           icon: 'Styles/Resolve/Guardian/Guardian.png' }
    ],
    rows: [
      // Strength: [8446, 8463, 8401]
      [
        { id: 8446, name: 'Demolir',           icon: 'Styles/Resolve/Demolish/Demolish.png' },
        { id: 8463, name: 'Fonte de Vida',     icon: 'Styles/Resolve/FontOfLife/FontOfLife.png' },
        { id: 8401, name: 'Golpe de Escudo',   icon: 'Styles/Resolve/MirrorShell/MirrorShell.png' }
      ],
      // Resistance: [8429, 8444, 8473]
      [
        { id: 8429, name: 'Condicionamento',   icon: 'Styles/Resolve/Conditioning/Conditioning.png' },
        { id: 8444, name: 'Vento Revigorante', icon: 'Styles/Resolve/SecondWind/SecondWind.png' },
        { id: 8473, name: 'Osso Revestido',    icon: 'Styles/Resolve/BonePlating/BonePlating.png' }
      ],
      // Vitality: [8451, 8453, 8242]
      [
        { id: 8451, name: 'Crescimento Excessivo', icon: 'Styles/Resolve/Overgrowth/Overgrowth.png' },
        { id: 8453, name: 'Revitalizar',            icon: 'Styles/Resolve/Revitalize/Revitalize.png' },
        { id: 8242, name: 'Inabalável',             icon: 'Styles/Sorcery/Unflinching/Unflinching.png' }
      ]
    ]
  },

  8300: {
    id: 8300,
    name: 'Inspiração',
    keystones: [
      { id: 8351, name: 'Aprimoramento Glacial', icon: 'Styles/Inspiration/GlacialAugment/GlacialAugment.png' },
      { id: 8360, name: 'Livro de Feitiços',     icon: 'Styles/Inspiration/UnsealedSpellbook/UnsealedSpellbook.png' },
      { id: 8369, name: 'Primeiro Ataque',        icon: 'Styles/Inspiration/FirstStrike/FirstStrike.png' }
    ],
    rows: [
      // Contraptions: [8306, 8304, 8321] — Cash Back (8321) entrou, Triple Tonic saiu
      [
        { id: 8306, name: 'Flashtração Hextec', icon: 'Styles/Inspiration/HextechFlashtraption/HextechFlashtraption.png' },
        { id: 8304, name: 'Calçados Mágicos',   icon: 'Styles/Inspiration/MagicalFootwear/MagicalFootwear.png' },
        { id: 8321, name: 'Dinheiro de Volta',  icon: 'Styles/Inspiration/CashBack/CashBack2.png' }
      ],
      // Tomorrow: [8313, 8352, 8345]
      [
        { id: 8313, name: 'Tônico Triplo',       icon: 'Styles/Inspiration/PerfectTiming/AlchemistCabinet.png' },
        { id: 8352, name: 'Tônico de Distorção', icon: 'Styles/Inspiration/TimeWarpTonic/TimeWarpTonic.png' },
        { id: 8345, name: 'Entrega de Biscoitos',icon: 'Styles/Inspiration/BiscuitDelivery/BiscuitDelivery.png' }
      ],
      // Beyond: [8347, 8410, 8316] — Jack of All Trades (8316) entrou com novo ícone
      [
        { id: 8347, name: 'Perspicácia Cósmica',      icon: 'Styles/Inspiration/CosmicInsight/CosmicInsight.png' },
        { id: 8410, name: 'Velocidade de Aproximação', icon: 'Styles/Resolve/ApproachVelocity/ApproachVelocity.png' },
        { id: 8316, name: 'Coringa',                   icon: 'Styles/Inspiration/JackOfAllTrades/JackofAllTrades2.png' }
      ]
    ]
  }
};

// Season 16 shard structure (offense / flex / defense)
export const SHARDS_ROWS = [
  [
    { id: 5008, name: 'Força Adaptativa (+9)',       icon: 'statmods/statmodsadaptiveforceicon.png' },
    { id: 5005, name: 'Velocidade de Ataque (+10%)', icon: 'statmods/statmodsattackspeedicon.png' },
    { id: 5007, name: 'Aceleração de Habilidade (+8)',icon: 'statmods/statmodscdroverflowicon.png' }
  ],
  [
    { id: 5008, name: 'Força Adaptativa (+9)',        icon: 'statmods/statmodsadaptiveforceicon.png' },
    { id: 5010, name: 'Velocidade de Movimento (+2%)',icon: 'statmods/statmodsmovementspeedicon.png' },
    { id: 5001, name: 'Vida Escalável',               icon: 'statmods/statmodshealthscalingicon.png' }
  ],
  [
    { id: 5011, name: 'Vida Flat (+65)',   icon: 'statmods/statmodshealthplusicon.png' },
    { id: 5013, name: 'Tenacidade (+10%)',icon: 'statmods/statmodstenacityicon.png' },
    { id: 5001, name: 'Vida Escalável',   icon: 'statmods/statmodshealthscalingicon.png' }
  ]
];

// IDs obsoletos que o backend pode enviar de partidas antigas — mantidos só para compatibilidade
export const RUNE_ID_ALIASES: Record<number, number> = {
  // Season 14 — Precision row[0] Absorb Life usava ID diferente
  9101: 9101,
};

export const normalizeRuneId = (id: number): number => RUNE_ID_ALIASES[id] ?? id;

export const getFullUrl = (path: string) => {
  if (!path) return '';
  if (path.startsWith('http')) return path;
  return `https://raw.communitydragon.org/latest/plugins/rcp-be-lol-game-data/global/default/v1/perk-images/${path.toLowerCase()}`;
};

export const getTreeHeaderIcon = (id: number) => {
  const map: Record<number, string> = {
    8000: 'styles/7201_precision.png',
    8100: 'styles/7200_domination.png',
    8200: 'styles/7202_sorcery.png',
    8300: 'styles/7203_whimsy.png',
    8400: 'styles/7204_resolve.png'
  };
  return getFullUrl(map[id] || map[8000]);
};
