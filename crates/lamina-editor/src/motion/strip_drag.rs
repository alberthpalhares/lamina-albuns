//! Reordenar na faixa de lâminas (spec FX, regras 1 a 5): as contas do arrasto, pelas larguras
//! reais de cada miniatura. A tela desenha; aqui ficam o vão, quanto cada vizinha corre, onde a
//! miniatura pousa e se soltar muda alguma coisa.

use super::drag::reorder;

/// O espaço entre duas miniaturas da faixa, em px lógicos.
pub const STRIP_SPACING: f64 = 10.0;

/// A borda esquerda de cada miniatura, a partir do começo da fileira.
pub fn strip_lefts(widths: &[f64]) -> Vec<f64> {
    let mut x = 0.0;
    widths
        .iter()
        .map(|w| {
            let left = x;
            x += w + STRIP_SPACING;
            left
        })
        .collect()
}

/// O vão (0 = antes da primeira; o número de lâminas = depois da última) para a miniatura `from`
/// levantada com o meio em `center`: conta as outras cujo meio ficou à esquerda (regra 2).
pub fn strip_gap(widths: &[f64], from: usize, center: f64) -> usize {
    let lefts = strip_lefts(widths);
    let before = (0..widths.len())
        .filter(|&i| i != from && lefts[i] + widths[i] / 2.0 < center)
        .count();
    if before >= from { before + 1 } else { before }
}

/// Quanto a miniatura `i` corre, em px, com a `from` levantada sobre o vão `gap`: o tamanho da
/// arrastada mais o espaço, para a esquerda ou para a direita (regra 2).
pub fn strip_shift(widths: &[f64], from: usize, gap: usize, i: usize) -> f64 {
    let step = widths.get(from).copied().unwrap_or(0.0) + STRIP_SPACING;
    let units = reorder(from, gap, widths.len())
        .shift
        .get(i)
        .copied()
        .unwrap_or(0);
    f64::from(units) * step
}

/// Onde a miniatura `from` pousa (a borda esquerda) no vão `gap` (regra 3): o mesmo lugar que ela
/// terá na fileira depois de mudar de ordem.
pub fn strip_landing(widths: &[f64], from: usize, gap: usize) -> f64 {
    let to = reorder(from, gap, widths.len()).to;
    let mut order: Vec<usize> = (0..widths.len()).filter(|&i| i != from).collect();
    order.insert(to.min(order.len()), from);
    let moved: Vec<f64> = order.iter().map(|&i| widths[i]).collect();
    strip_lefts(&moved)[to.min(order.len().saturating_sub(1))]
}

/// Para onde a lâmina vai ao soltar (regras 3 e 5): `None` quando soltou fora da faixa ou no mesmo
/// lugar.
pub fn strip_drop(from: usize, gap: usize, count: usize, inside: bool) -> Option<usize> {
    let to = reorder(from, gap, count).to;
    (inside && from < count && to != from).then_some(to)
}
