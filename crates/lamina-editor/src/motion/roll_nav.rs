//! Navegar e rolar no rolo de lâminas (spec RL, regras 11, 12 e 14): como ir para outra lâmina,
//! o destino da rolagem e a pílula da lâmina atual fora de vista.

use std::ops::RangeInclusive;

use super::navigation::Transition;

/// Até esta distância do centro, ir para outra lâmina desliza o rolo (regra 12).
pub const NEAR: f64 = 2.0;

/// Como o rolo vai para outra lâmina (regra 12).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RollMove {
    /// Nada muda (a mesma lâmina).
    Stay,
    /// Troca sem animação: navegação seguida, outro álbum, "Reduzir movimento".
    Cut,
    /// O rolo desliza até a lâmina ficar no centro.
    Slide,
    /// Como hoje: a atual esmaece e a nova entra deslizando 32 px (1 = vem da direita).
    Fade(i32),
}

/// O movimento para ir à lâmina `to`, contando a distância a partir do que se vê no centro
/// (`seen`, o destino da rolagem), com a transição decidida pelo `Navigator`.
pub fn roll_move(seen: f64, to: usize, transition: Transition) -> RollMove {
    #[allow(clippy::cast_precision_loss)]
    let near = (to as f64 - seen).abs() <= NEAR + 1e-9;
    match transition {
        Transition::Stay => RollMove::Stay,
        Transition::Cut => RollMove::Cut,
        Transition::Forward | Transition::Backward if near => RollMove::Slide,
        Transition::Forward => RollMove::Fade(1),
        Transition::Backward => RollMove::Fade(-1),
    }
}

/// A pílula da lâmina atual quando ela saiu de vista (regra 14).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pill {
    /// A lâmina atual ficou à esquerda.
    pub left: bool,
    pub label: String,
}

/// A pílula, se a lâmina `current` não está entre as `visible`.
pub fn pill(current: usize, visible: &RangeInclusive<usize>) -> Option<Pill> {
    let n = current + 1;
    if current < *visible.start() {
        Some(Pill {
            left: true,
            label: format!("← Lâmina {n}"),
        })
    } else if current > *visible.end() {
        Some(Pill {
            left: false,
            label: format!("Lâmina {n} →"),
        })
    } else {
        None
    }
}

/// O destino da rolagem do rolo: giros seguidos somam; rolar à mão para de seguir a lâmina atual
/// até a próxima navegação (regra 11).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RollScroll {
    /// A posição de destino (a lâmina no centro, com fração).
    pub target: f64,
    /// O rolo acompanha a lâmina atual.
    pub follow: bool,
}

impl Default for RollScroll {
    fn default() -> Self {
        RollScroll {
            target: 0.0,
            follow: true,
        }
    }
}

impl RollScroll {
    /// A roda ou o touchpad andaram `delta` pixels (positivo = para trás, como no Slint) num rolo
    /// de passo `step` com `count` lâminas: o destino vai da primeira ao "+ Lâmina nova".
    pub fn wheel(&mut self, delta: f64, step: f64, count: usize) {
        self.nudge(-delta / step.max(1.0), count);
    }

    /// Anda `spreads` lâminas (a rolagem nas bordas), sem seguir a lâmina atual.
    pub fn nudge(&mut self, spreads: f64, count: usize) {
        #[allow(clippy::cast_precision_loss)]
        let end = count as f64;
        self.target = (self.target + spreads).clamp(0.0, end);
        self.follow = false;
    }

    /// Volta a acompanhar a lâmina `index` (navegar, ou o clique na pílula).
    pub fn follow(&mut self, index: usize) {
        #[allow(clippy::cast_precision_loss)]
        let at = index as f64;
        self.target = at;
        self.follow = true;
    }
}
