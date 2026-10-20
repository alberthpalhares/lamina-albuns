//! Ir para outra lâmina (spec MV, seção 4 e regra 9): a lâmina nova entra deslizando na direção
//! da navegação; navegar seguido (menos de 200 ms depois da anterior), abrir outro álbum ou
//! "Reduzir movimento" trocam sem animação.

use std::time::Duration;

use super::slots::Place;

/// Navegação a menos disto da anterior troca sem animação (Page Down segurado).
pub const RAPID: Duration = Duration::from_millis(200);

/// Como a lâmina grande muda.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Transition {
    /// A mesma lâmina (ou a primeira exibição): nada desliza.
    Stay,
    /// Outra lâmina, sem animação.
    Cut,
    /// A lâmina nova vem da direita (para a frente) ou da esquerda (para trás).
    Forward,
    Backward,
}

/// A lâmina mostrada por último, para decidir a próxima transição.
#[derive(Debug, Default)]
pub struct Navigator {
    last: Option<(Place, usize)>,
    moved_at: Option<Duration>,
}

impl Navigator {
    /// A lâmina grande passa a mostrar a lâmina `index` (em `place`) no instante `at` (tempo da
    /// janela). `reduce`: "Reduzir movimento" ligado.
    pub fn arrive(&mut self, place: Place, index: usize, at: Duration, reduce: bool) -> Transition {
        let before = self.last.replace((place, index));
        let Some((old_place, old_index)) = before else {
            return Transition::Stay;
        };
        if old_place == place {
            return Transition::Stay;
        }
        let rapid = self
            .moved_at
            .replace(at)
            .is_some_and(|t| at.saturating_sub(t) < RAPID);
        let other_album = old_place.0 != place.0;
        if reduce || rapid || other_album {
            return Transition::Cut;
        }
        if index >= old_index {
            Transition::Forward
        } else {
            Transition::Backward
        }
    }
}
