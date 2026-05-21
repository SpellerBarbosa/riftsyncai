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

export const TREE_STRUCTURES: Record<number, TreeStructure> = {
  8000: {
    id: 8000,
    name: 'Precisão',
    keystones: [
      { id: 8005, name: 'Pressione o Ataque', icon: 'Styles/Precision/PressTheAttack/PressTheAttack.png' },
      { id: 8008, name: 'Ritmo Fatal', icon: 'Styles/Precision/LethalTempo/LethalTempoTemp.png' },
      { id: 8021, name: 'Agilidade nos Pés', icon: 'Styles/Precision/FleetFootwork/FleetFootwork.png' },
      { id: 8010, name: 'Conquistador', icon: 'Styles/Precision/Conqueror/Conqueror.png' }
    ],
    rows: [
      [
        { id: 9101, name: 'Sobrecura', icon: 'Styles/Precision/Overheal.png' },
        { id: 9111, name: 'Triunfo', icon: 'Styles/Precision/Triumph.png' },
        { id: 8009, name: 'Presença de Espírito', icon: 'Styles/Precision/PresenceOfMind/PresenceOfMind.png' }
      ],
      [
        { id: 9104, name: 'Lenda: Alacridade', icon: 'Styles/Precision/LegendAlacrity/LegendAlacrity.png' },
        { id: 9105, name: 'Lenda: Tenacidade', icon: 'Styles/Precision/LegendTenacity/LegendTenacity.png' },
        { id: 9103, name: 'Lenda: Linhagem', icon: 'Styles/Precision/LegendBloodline/LegendBloodline.png' }
      ],
      [
        { id: 8014, name: 'Golpe de Misericórdia', icon: 'Styles/Precision/CoupDeGrace/CoupDeGrace.png' },
        { id: 8017, name: 'Dilacerar', icon: 'Styles/Precision/CutDown/CutDown.png' },
        { id: 8299, name: 'Até a Morte', icon: 'Styles/Sorcery/LastStand/LastStand.png' },
        { id: 8992, name: 'Toque Ígneo', icon: 'Styles/Precision/EyeballCollection/EyeballCollection.png' }
      ]
    ]
  },
  8100: {
    id: 8100,
    name: 'Dominação',
    keystones: [
      { id: 8112, name: 'Eletrocutar', icon: 'Styles/Domination/Electrocute/Electrocute.png' },
      { id: 8124, name: 'Predador', icon: 'Styles/Domination/Predator/Predator.png' },
      { id: 8128, name: 'Colheita Sombria', icon: 'Styles/Domination/DarkHarvest/DarkHarvest.png' },
      { id: 9923, name: 'Chuva de Lâminas', icon: 'Styles/Domination/HailOfBlades/HailOfBlades.png' }
    ],
    rows: [
      [
        { id: 8126, name: 'Golpe Desleal', icon: 'Styles/Domination/CheapShot/CheapShot.png' },
        { id: 8139, name: 'Gosto de Sangue', icon: 'Styles/Domination/TasteOfBlood/GreenTerror_TasteOfBlood.png' },
        { id: 8143, name: 'Impacto Repentino', icon: 'Styles/Domination/SuddenImpact/SuddenImpact.png' }
      ],
      [
        { id: 8136, name: 'Sentinela Zumbi', icon: 'Styles/Domination/ZombieWard/ZombieWard.png' },
        { id: 8120, name: 'Poro Fantasma', icon: 'Styles/Domination/GhostPoro/GhostPoro.png' },
        { id: 8138, name: 'Coleção de Olhos', icon: 'Styles/Domination/EyeballCollection/EyeballCollection.png' }
      ],
      [
        { id: 8135, name: 'Caçador de Tesouros', icon: 'Styles/Domination/TreasureHunter/TreasureHunter.png' },
        { id: 8134, name: 'Caçador Incansável', icon: 'Styles/Domination/RelentlessHunter/RelentlessHunter.png' },
        { id: 8105, name: 'Caçadora Ardilosa', icon: 'Styles/Domination/IngeniousHunter/IngeniousHunter.png' },
        { id: 8106, name: 'Caçadora Suprema', icon: 'Styles/Domination/UltimateHunter/UltimateHunter.png' }
      ]
    ]
  },
  8200: {
    id: 8200,
    name: 'Feitiçaria',
    keystones: [
      { id: 8214, name: 'Invocação: Aery', icon: 'Styles/Sorcery/SummonAery/SummonAery.png' },
      { id: 8229, name: 'Cometa Arcano', icon: 'Styles/Sorcery/ArcaneComet/ArcaneComet.png' },
      { id: 8230, name: 'Ímpeto Gradual', icon: 'Styles/Sorcery/PhaseRush/PhaseRush.png' }
    ],
    rows: [
      [
        { id: 8224, name: 'Orbe Anulador', icon: 'Styles/Sorcery/NullifyingOrb/Pokeshield.png' },
        { id: 8226, name: 'Faixa de Fluxo de Mana', icon: 'Styles/Sorcery/ManaflowBand/ManaflowBand.png' },
        { id: 8275, name: 'Manto de Nimbus', icon: 'perk-images/Styles/Sorcery/NimbusCloak/6361.png' }
      ],
      [
        { id: 8210, name: 'Transcendência', icon: 'Styles/Sorcery/Transcendence/Transcendence.png' },
        { id: 8234, name: 'Celeridade', icon: 'Styles/Sorcery/Celerity/CelerityTemp.png' },
        { id: 8233, name: 'Foco Absoluto', icon: 'Styles/Sorcery/AbsoluteFocus/AbsoluteFocus.png' }
      ],
      [
        { id: 8237, name: 'Chamuscar', icon: 'Styles/Sorcery/Scorch/Scorch.png' },
        { id: 8232, name: 'Tempestade Crescente', icon: 'Styles/Sorcery/GatheringStorm/GatheringStorm.png' },
        { id: 8236, name: 'Caminhar Sobre as Águas', icon: 'Styles/Sorcery/Waterwalking/Waterwalking.png' }
      ]
    ]
  },
  8400: {
    id: 8400,
    name: 'Determinação',
    keystones: [
      { id: 8437, name: 'Aperto Morto-Vivo', icon: 'Styles/Resolve/GraspOfTheUndying/GraspOfTheUndying.png' },
      { id: 8439, name: 'Pós-choque', icon: 'Styles/Resolve/VeteranAftershock/VeteranAftershock.png' },
      { id: 8465, name: 'Guardião', icon: 'Styles/Resolve/Guardian/Guardian.png' }
    ],
    rows: [
      [
        { id: 8446, name: 'Demolir', icon: 'Styles/Resolve/Demolish/Demolish.png' },
        { id: 8463, name: 'Fonte de Vida', icon: 'Styles/Resolve/FontOfLife/FontOfLife.png' },
        { id: 8401, name: 'Golpe de Escudo', icon: 'Styles/Resolve/MirrorShell/MirrorShell.png' }
      ],
      [
        { id: 8429, name: 'Condicionamento', icon: 'Styles/Resolve/Conditioning/Conditioning.png' },
        { id: 8444, name: 'Vento Revigorante', icon: 'Styles/Resolve/SecondWind/SecondWind.png' },
        { id: 8473, name: 'Osso Revestido', icon: 'Styles/Resolve/BonePlating/BonePlating.png' }
      ],
      [
        { id: 8451, name: 'Crescimento Excessivo', icon: 'Styles/Resolve/Overgrowth/Overgrowth.png' },
        { id: 8453, name: 'Revitalizar', icon: 'Styles/Resolve/Revitalize/Revitalize.png' },
        { id: 8242, name: 'Inabalável', icon: 'Styles/Sorcery/Unflinching/Unflinching.png' }
      ]
    ]
  },
  8300: {
    id: 8300,
    name: 'Inspiração',
    keystones: [
      { id: 8351, name: 'Aprimoramento Glacial', icon: 'Styles/Inspiration/GlacialAugment/GlacialAugment.png' },
      { id: 8360, name: 'Livro de Feitiços', icon: 'Styles/Inspiration/UnsealedSpellbook/UnsealedSpellbook.png' },
      { id: 8369, name: 'Primeiro Ataque', icon: 'Styles/Inspiration/FirstStrike/FirstStrike.png' }
    ],
    rows: [
      [
        { id: 8306, name: 'Flashtração Hextec', icon: 'Styles/Inspiration/HextechFlashtraption/HextechFlashtraption.png' },
        { id: 8304, name: 'Calçados Mágicos', icon: 'Styles/Inspiration/MagicalFootwear/MagicalFootwear.png' },
        { id: 8313, name: 'Tônico Triplo', icon: 'Styles/Inspiration/PerfectTiming/AlchemistCabinet.png' }
      ],
      [
        { id: 8321, name: 'Mercado do Futuro', icon: 'Styles/Inspiration/FuturesMarket/FuturesMarket.png' },
        { id: 8316, name: 'Pulverizador de Catapulta', icon: 'Styles/Inspiration/MinionDematerializer/MinionDematerializer.png' },
        { id: 8345, name: 'Entrega de Biscoitos', icon: 'Styles/Inspiration/BiscuitDelivery/BiscuitDelivery.png' }
      ],
      [
        { id: 8347, name: 'Perspicácia Cósmica', icon: 'Styles/Inspiration/CosmicInsight/CosmicInsight.png' },
        { id: 8410, name: 'Velocidade de Aproximação', icon: 'Styles/Resolve/ApproachVelocity/ApproachVelocity.png' },
        { id: 8352, name: 'Tônico de Distorção', icon: 'Styles/Inspiration/TimeWarpTonic/TimeWarpTonic.png' }
      ]
    ]
  }
};

export const SHARDS_ROWS = [
  [
    { id: 5008, name: 'Força Adaptativa (+9)', icon: 'statmods/statmodsadaptiveforceicon.png' },
    { id: 5005, name: 'Velocidade de Ataque (+10%)', icon: 'statmods/statmodsattackspeedicon.png' },
    { id: 5007, name: 'Aceleração de Habilidade (+8)', icon: 'statmods/statmodscdroverflowicon.png' }
  ],
  [
    { id: 5008, name: 'Força Adaptativa (+9)', icon: 'statmods/statmodsadaptiveforceicon.png' },
    { id: 5010, name: 'Velocidade de Movimento (+2%)', icon: 'statmods/statmodsmovementspeedicon.png' },
    { id: 5001, name: 'Vida Escalável', icon: 'statmods/statmodshealthscalingicon.png' }
  ],
  [
    { id: 5001, name: 'Vida Escalável', icon: 'statmods/statmodshealthscalingicon.png' },
    { id: 5011, name: 'Vida Flat (+65)', icon: 'statmods/statmodshealthplusicon.png' },
    { id: 5013, name: 'Tenacidade (+10%)', icon: 'statmods/statmodstenacityicon.png' }
  ]
];

/**
 * Maps alternative/renamed rune IDs to their canonical visual equivalent in TREE_STRUCTURES.
 * These are IDs used by the API/database that correspond to the same visual slot
 * but have different numeric IDs across patches.
 */
export const RUNE_ID_ALIASES: Record<number, number> = {
  // Domination row[1] — Observer-type slot variants
  8137: 8136, // Sexto Sentido → Sentinela Zumbi slot
  8140: 8138, // Lembranças Aterrorizantes → Coleção de Olhos slot  
  8141: 8120, // Sentinela Profunda → Poro Fantasma slot
  // Precision row[2] — Last Stand variants
  8992: 8299, // Toque Ígneo → Até a Morte slot
};

/**
 * Normalize a rune ID — if it's a known alias, return the canonical visual ID.
 */
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
