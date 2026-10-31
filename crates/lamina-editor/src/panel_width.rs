//! A largura do painel da direita (spec FX, regras 13 a 15): arrastada pela borda esquerda, entre
//! 260 px e 45% da janela, e 340 px (spec DJ, regra 14) no clique duplo ou sem nada guardado.

/// A largura de sempre, em px lógicos.
pub const PANEL_WIDTH: u32 = 340;
/// A menor largura do painel.
pub const PANEL_MIN: u32 = 260;

/// A largura mostrada numa janela de `window` px, a partir da guardada (`None` = a de sempre).
pub fn panel_width(stored: Option<u32>, window: u32) -> u32 {
    let max = (u64::from(window) * 45 / 100).max(u64::from(PANEL_MIN));
    let max = u32::try_from(max).unwrap_or(u32::MAX);
    stored.unwrap_or(PANEL_WIDTH).clamp(PANEL_MIN, max)
}
