//! O zoom das miniaturas, a altura do painel de fotos (a divisória) e a rolagem do laço perto da
//! borda (spec PF, regras 4, 5 e 9).

use super::{GRID_GAP, Grid};

/// O zoom das miniaturas: de 64 a 240 px, 96 sem o campo em `janela.json`.
pub const ZOOM_MIN: u32 = 64;
pub const ZOOM_MAX: u32 = 240;
pub const ZOOM_DEFAULT: u32 = 96;
/// O controle anda de 8 em 8 e a Ctrl+roda de 16 em 16 (regra 4).
pub const ZOOM_STEP: u32 = 8;
pub const ZOOM_WHEEL_STEP: u32 = 16;
/// O laço rola a grade a esta distância da borda, até este tanto por quadro (regra 9).
pub const LASSO_EDGE: f32 = 24.0;
pub const LASSO_SPEED: f32 = 20.0;
/// A área da lâmina nunca fica menor que isto, e o painel nunca passa desta parte da janela.
pub const STAGE_MIN: f32 = 160.0;
pub const HEIGHT_SHARE: f32 = 0.7;

/// O zoom preso à faixa de 64 a 240 px.
pub fn clamp_zoom(zoom: i64) -> u32 {
    let clamped = zoom.clamp(i64::from(ZOOM_MIN), i64::from(ZOOM_MAX));
    u32::try_from(clamped).unwrap_or(ZOOM_DEFAULT)
}

/// O zoom depois de um passo da Ctrl+roda (`up`: miniaturas maiores), parando nas pontas.
pub fn wheel_zoom(zoom: u32, up: bool) -> u32 {
    let step = i64::from(ZOOM_WHEEL_STEP);
    clamp_zoom(i64::from(zoom) + if up { step } else { -step })
}

/// A rolagem depois do zoom pela Ctrl+roda: a foto embaixo do ponteiro (`pointer`, no painel)
/// continua embaixo dele, na mesma altura dentro da linha (regra 4).
pub fn zoom_at_pointer(old: &Grid, new: &Grid, scroll: f32, pointer: (f32, f32)) -> f32 {
    let (h, y) = (old.row_height(), scroll + pointer.1);
    let row = old.top_row(y);
    let index = (row * old.columns + old.column_at(pointer.0)).min(old.count.saturating_sub(1));
    let within = ((y - row as f32 * h) / h).clamp(0.0, 1.0);
    let new_h = new.row_height();
    (new.row_of(index) as f32 * new_h + within * new_h - pointer.1).max(0.0)
}

/// A rolagem depois do zoom pelo controle: a linha que tinha a primeira foto visível fica no
/// topo (regra 4). `height` é a altura do painel.
pub fn zoom_keeping_top(old: &Grid, new: &Grid, scroll: f32, height: f32) -> f32 {
    let first = old.row_range(old.top_row(scroll)).start;
    new.scroll_to(first, height)
}

/// Quanto o laço rola a grade por quadro com o ponteiro em `y` num painel de altura `height`
/// (regra 9): zero longe das bordas; perto, mais rápido quanto mais perto, até 20 px.
pub fn lasso_scroll(y: f32, height: f32) -> f32 {
    let speed = |depth: f32| LASSO_SPEED * (depth / LASSO_EDGE).min(1.0);
    if y > height - LASSO_EDGE {
        speed(y - (height - LASSO_EDGE))
    } else if y < LASSO_EDGE {
        -speed(LASSO_EDGE - y)
    } else {
        0.0
    }
}

/// A altura de `rows` linhas no zoom `zoom` (8 + k · H).
pub fn rows_height(rows: usize, zoom: u32) -> f32 {
    let grid = Grid::new(1, 10_000.0, zoom);
    GRID_GAP + rows as f32 * grid.row_height()
}

/// A altura padrão do painel: duas linhas no zoom atual (regra 5).
pub fn default_height(zoom: u32) -> f32 {
    rows_height(2, zoom)
}

/// Os limites da altura do painel numa janela de altura `window`, em que `chrome` é o que não é
/// nem a lâmina nem a grade (barras e cabeçalhos): no mínimo uma linha; no máximo 70% da janela
/// sem deixar a lâmina com menos de 160 px. Se brigarem, vale uma linha.
pub fn height_limits(window: f32, chrome: f32, zoom: u32) -> (f32, f32) {
    let min = rows_height(1, zoom);
    let max = (HEIGHT_SHARE * window).min(window - chrome - STAGE_MIN);
    (min, max.max(min))
}

/// A altura que a grade mostra: a guardada (ou a padrão) dentro dos limites da janela de agora.
pub fn shown_height(stored: Option<f32>, window: f32, chrome: f32, zoom: u32) -> f32 {
    let (min, max) = height_limits(window, chrome, zoom);
    stored
        .unwrap_or_else(|| default_height(zoom))
        .clamp(min, max)
}

/// Uma altura da tela em px inteiros, como fica em `janela.json`.
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
pub fn whole_px(height: f32) -> u32 {
    height.max(0.0).round() as u32
}
