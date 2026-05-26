/// Detecção de ícones inimigos no minimapa do LoL via análise de pixels.
///
/// Funcionamento:
///   1. Captura apenas a região do minimapa (canto inferior direito, ~11% da largura)
///   2. Varre os pixels em busca da cor vermelho-laranja característica dos ícones inimigos
///   3. Agrupa pixels próximos em clusters (cada cluster ≈ um ícone inimigo)
///   4. Mapeia a posição normalizada do cluster para uma zona do mapa (top/mid/bot/rio/pit)
///
/// Custo: ~1-3ms por execução (captura GDI + varredura de ~40k pixels).
/// Executado apenas quando um inimigo reaparece após névoa — não é contínuo.

use screenshots::Screen;

// ── Parâmetros de detecção ────────────────────────────────────────────────────

/// Pixels mínimos num cluster para ser considerado um ícone (filtra ruído de 1-2px).
const MIN_CLUSTER_PIXELS: u32 = 4;

/// Raio de agrupamento: pixels dentro desta distância pertencem ao mesmo cluster.
const CLUSTER_RADIUS_PX: u32 = 10;

/// Margem de borda ignorada (evita artefatos da moldura do minimapa).
const EDGE_MARGIN: u32 = 6;

// ── Captura de tela ───────────────────────────────────────────────────────────

/// Captura a região do minimapa e retorna (pixels_rgba, largura, altura).
/// O minimapa ocupa ~11,1% da largura da tela e fica posicionado no canto
/// inferior direito com margem de ~4px da borda.
fn capture_minimap_pixels() -> Option<(Vec<u8>, u32, u32)> {
    let screens = Screen::all().ok()?;
    let screen = screens
        .iter()
        .find(|s| s.display_info.is_primary)
        .or_else(|| screens.first())?;

    let sw = screen.display_info.width;
    let sh = screen.display_info.height;

    // Tamanho do minimapa: proporção padrão do LoL (~11,1% da largura)
    let mm_size = (sw as f32 * 0.111).round() as u32;
    let mm_x = sw.saturating_sub(mm_size + 4) as i32;
    let mm_y = sh.saturating_sub(mm_size + 4) as i32;

    let image = screen.capture_area(mm_x, mm_y, mm_size, mm_size).ok()?;
    let width  = image.width();
    let height = image.height();
    let pixels = image.into_raw(); // formato RGBA, 4 bytes por pixel
    Some((pixels, width, height))
}

// ── Detecção de cor ───────────────────────────────────────────────────────────

/// Retorna true se o pixel RGBA tem a cor vermelho-laranja dos ícones inimigos no LoL.
/// Faixa calibrada para o tom padrão: R:190-255, G:50-110, B:30-90.
#[inline]
fn is_enemy_red(r: u8, g: u8, b: u8) -> bool {
    let r = r as i32;
    let g = g as i32;
    let b = b as i32;
    r > 180          // vermelho forte
        && g < 120   // baixo verde
        && b < 110   // baixo azul
        && r - g > 80 // vermelho dominante sobre verde
        && r - b > 80 // vermelho dominante sobre azul
}

// ── Clustering ────────────────────────────────────────────────────────────────

struct Cluster {
    sum_x: u64,
    sum_y: u64,
    count: u32,
}

impl Cluster {
    fn centroid(&self) -> (u32, u32) {
        (
            (self.sum_x / self.count as u64) as u32,
            (self.sum_y / self.count as u64) as u32,
        )
    }
}

/// Varre o minimapa, encontra pixels vermelhos e os agrupa em clusters.
/// Retorna lista de posições normalizadas (nx, ny) onde 0.0 = topo/esquerda, 1.0 = base/direita.
fn detect_clusters(pixels: &[u8], width: u32, height: u32) -> Vec<(f32, f32)> {
    let mut red_pts: Vec<(u32, u32)> = Vec::new();

    // Varredura ignorando a borda (EDGE_MARGIN pixels de cada lado)
    for y in EDGE_MARGIN..(height.saturating_sub(EDGE_MARGIN)) {
        for x in EDGE_MARGIN..(width.saturating_sub(EDGE_MARGIN)) {
            let i = ((y * width + x) * 4) as usize;
            if i + 2 < pixels.len() && is_enemy_red(pixels[i], pixels[i + 1], pixels[i + 2]) {
                red_pts.push((x, y));
            }
        }
    }

    if red_pts.is_empty() {
        return Vec::new();
    }

    // Agrupamento greedy: cada ponto é adicionado ao cluster mais próximo dentro do raio,
    // ou cria um novo cluster. O centroide é recalculado iterativamente.
    let mut clusters: Vec<Cluster> = Vec::new();

    'outer: for (px, py) in &red_pts {
        for c in &mut clusters {
            let (cx, cy) = c.centroid();
            if px.abs_diff(cx) <= CLUSTER_RADIUS_PX && py.abs_diff(cy) <= CLUSTER_RADIUS_PX {
                c.sum_x += *px as u64;
                c.sum_y += *py as u64;
                c.count += 1;
                continue 'outer;
            }
        }
        clusters.push(Cluster {
            sum_x: *px as u64,
            sum_y: *py as u64,
            count: 1,
        });
    }

    clusters
        .iter()
        .filter(|c| c.count >= MIN_CLUSTER_PIXELS)
        .map(|c| {
            let (cx, cy) = c.centroid();
            (cx as f32 / width as f32, cy as f32 / height as f32)
        })
        .collect()
}

// ── Mapeamento de zona ────────────────────────────────────────────────────────

/// Mapeia posição normalizada do minimapa para zona do jogo.
///
/// Orientação do minimapa do LoL (imagem capturada):
///   ny=0 → topo da imagem = topo do mapa (base vermelha)
///   ny=1 → base da imagem = base do mapa (base azul)
///   nx=0 → esquerda = rota do topo / Barão
///   nx=1 → direita = rota bot / Dragão
pub fn pixel_to_zone(nx: f32, ny: f32) -> &'static str {
    // Pit do Dragão — canto inferior direito
    if nx > 0.60 && ny > 0.58 {
        return "perto do Dragão";
    }
    // Pit do Barão/Arauto — canto superior esquerdo
    if nx < 0.38 && ny < 0.40 {
        return "perto do Barão";
    }
    // Rota do topo — borda esquerda e faixa superior
    if nx < 0.22 || (nx < 0.38 && ny < 0.20) {
        return "no top";
    }
    // Rota bot — borda direita e faixa inferior
    if nx > 0.78 || (nx > 0.62 && ny > 0.80) {
        return "no bot";
    }
    // Mid lane — faixa diagonal ±0.18 em torno da anti-diagonal (nx + ny ≈ 1.0)
    if (nx + ny - 1.0).abs() < 0.18 {
        return "no meio";
    }
    // Rio/selva (todo o restante)
    "no rio"
}

// ── API pública ───────────────────────────────────────────────────────────────

/// Captura o minimapa e tenta identificar a zona onde o inimigo foi avistado.
///
/// `role` é usado como heurística de desempate quando múltiplos clusters são
/// detectados: prefere o cluster na zona esperada para aquele role.
///
/// Retorna `None` se:
/// - a captura falhar (LoL não está aberto / tela não acessível)
/// - nenhum cluster vermelho for encontrado (inimigo ainda em névoa de guerra)
pub fn find_enemy_zone(role: &str) -> Option<String> {
    let (pixels, width, height) = capture_minimap_pixels()?;
    let clusters = detect_clusters(&pixels, width, height);

    if clusters.is_empty() {
        return None;
    }

    // Zona esperada para o role (heurística de correspondência)
    let preferred = match role {
        "TOP"             => "no top",
        "MID"             => "no meio",
        "ADC" | "SUPPORT" => "no bot",
        "JUNGLE"          => "no rio",
        _                 => "",
    };

    // 1ª tentativa: cluster na zona esperada para o role
    if !preferred.is_empty() {
        for (nx, ny) in &clusters {
            let zone = pixel_to_zone(*nx, *ny);
            if zone == preferred {
                return Some(zone.to_string());
            }
        }
    }

    // 2ª tentativa: cluster mais próximo do centro esperado para o role
    // (caso o inimigo esteja ligeiramente fora da zona padrão, ex: roaming)
    let (nx, ny) = clusters[0]; // clusters já ordenados por posição de varredura
    Some(pixel_to_zone(nx, ny).to_string())
}
