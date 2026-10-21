//! As contas do rolo de lâminas (spec RL, regras 2, 9, 18, 19 e 21): a largura da lâmina, as
//! lâminas à vista, o que está sob o ponteiro e a rolagem nas bordas. Puras, em pixels lógicos da
//! área da lâmina; a janela só traduz. Navegar e rolar ficam em `roll_nav.rs`; o selo e a linha do
//! tempo, em `roll_timeline.rs`.

use std::ops::RangeInclusive;

/// Vão entre as lâminas do rolo, em pixels.
pub const ROLL_GAP: f64 = 24.0;
/// Margem em volta da lâmina (a de hoje), em pixels.
const MARGIN: f64 = 48.0;
/// Parte da largura da área que a lâmina ocupa quando há vizinhas (8% de cada lado para elas).
const SHARE: f64 = 0.84;
/// Abaixo desta largura, a área não mostra vizinhas.
pub const NARROW: f64 = 640.0;
/// Faixa das bordas que rola sozinha durante um arrasto, e a velocidade máxima (lâminas/s).
pub const EDGE_PX: f64 = 48.0;
pub const EDGE_SPEED: f64 = 2.0;
/// Quanto o vão invade cada lâmina vizinha para receber fotos (regra 20).
const GAP_REACH: f64 = 16.0;

/// A largura da lâmina e se há vizinhas.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RollWidth {
    pub width: f64,
    pub neighbors: bool,
}

/// A largura da lâmina numa área `area` (largura, altura), com `top` pixels ocupados em cima e a
/// proporção `aspect` (altura ÷ largura da lâmina) (regra 2).
pub fn roll_width(area: (f64, f64), top: f64, aspect: f64) -> RollWidth {
    let neighbors = area.0 >= NARROW;
    let share = if neighbors { area.0 * SHARE } else { area.0 };
    let by_height = (area.1 - MARGIN - top) / aspect.max(1e-9);
    RollWidth {
        width: (share - MARGIN).min(by_height).max(0.0),
        neighbors,
    }
}

/// O rolo como a janela o desenha: a área, o tamanho de cada lâmina e quantas há. A posição
/// (`pos`) é a lâmina no meio da área, com fração; o "+ Lâmina nova" fica na posição `count`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Roll {
    pub area: (f64, f64),
    pub width: f64,
    pub height: f64,
    pub count: usize,
}

impl Roll {
    /// Distância entre o começo de uma lâmina e o da seguinte.
    pub fn step(&self) -> f64 {
        self.width + ROLL_GAP
    }

    /// A borda esquerda da lâmina `index` com o rolo em `pos`.
    pub fn left(&self, index: usize, pos: f64) -> f64 {
        #[allow(clippy::cast_precision_loss)]
        let i = index as f64;
        self.area.0 / 2.0 - self.width / 2.0 + (i - pos) * self.step()
    }

    /// A borda de cima das lâminas (centralizadas na altura).
    pub fn top(&self) -> f64 {
        (self.area.1 - self.height) / 2.0
    }
}

/// As posições à vista (de 0 a `count`, que é o "+ Lâmina nova"), mesmo que só uma beirada.
pub fn visible_range(roll: &Roll, pos: f64) -> Option<RangeInclusive<usize>> {
    let seen = |i: usize| {
        let left = roll.left(i, pos);
        left < roll.area.0 && left + roll.width > 0.0
    };
    let first = (0..=roll.count).find(|i| seen(*i))?;
    let last = (first..=roll.count).take_while(|i| seen(*i)).last()?;
    Some(first..=last)
}

/// As lâminas que seguram textura na janela: as visíveis ±1 (regra 9).
pub fn textured(visible: &RangeInclusive<usize>, count: usize) -> RangeInclusive<usize> {
    let last = count.saturating_sub(1);
    visible.start().saturating_sub(1).min(last)..=(visible.end() + 1).min(last)
}

/// O que está sob o ponteiro no rolo (regra 21).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RollHit {
    /// Uma lâmina, com o ponto em frações da largura dela (como a lâmina grande).
    Spread { index: usize, fx: f64, fy: f64 },
    /// O vão antes da lâmina `n`: uma lâmina nova entraria como a `n`.
    Gap(usize),
    /// O "+ Lâmina nova", depois da última.
    New,
}

/// O que está no ponto (`x`, `y`) da área com o rolo em `pos`, ou nada.
pub fn roll_hit(roll: &Roll, pos: f64, (x, y): (f64, f64)) -> Option<RollHit> {
    let top = roll.top();
    if y < top || y > top + roll.height {
        return None;
    }
    let gap = (1..roll.count).find(|i| {
        let start = roll.left(*i, pos) - ROLL_GAP;
        x >= start - GAP_REACH && x <= start + ROLL_GAP + GAP_REACH
    });
    if let Some(i) = gap {
        return Some(RollHit::Gap(i));
    }
    let k = ((x - roll.left(0, pos)) / roll.step()).floor();
    if k < 0.0 {
        return None;
    }
    // Inteiro e não negativo; o que passa de `count` sai abaixo.
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let index = k as usize;
    let left = roll.left(index, pos);
    if index > roll.count || x > left + roll.width {
        return None;
    }
    if index == roll.count {
        return Some(RollHit::New);
    }
    let (fx, fy) = ((x - left) / roll.width, (y - top) / roll.width);
    Some(RollHit::Spread { index, fx, fy })
}

/// O vão mais perto do ponteiro em `x` (0 = antes da primeira; `count` = depois da última), para
/// reordenar pela alça (regra 18).
pub fn roll_gap(roll: &Roll, pos: f64, x: f64) -> usize {
    let first = roll.left(0, pos) - ROLL_GAP / 2.0;
    let k = ((x - first) / roll.step()).round().max(0.0);
    // Preso entre 0 e `count`.
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let gap = (k as usize).min(roll.count);
    gap
}

/// A velocidade da rolagem automática com o ponteiro em `x` numa área de largura `width`, em
/// lâminas por segundo (negativa para a esquerda): até 2 na borda, zero a 48 px dela (regra 19).
pub fn edge_speed(x: f64, width: f64) -> f64 {
    let ramp = |d: f64| EDGE_SPEED * (EDGE_PX - d.max(0.0)).max(0.0) / EDGE_PX;
    if width - x < EDGE_PX {
        ramp(width - x)
    } else if x < EDGE_PX {
        -ramp(x)
    } else {
        0.0
    }
}
