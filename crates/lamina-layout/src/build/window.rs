//! As sugestões que o Auto Build considera em cada lâmina, com as preferências do fotógrafo
//! (spec TL, regra 8): as primeiras da variedade mais os favoritos que vierem depois delas, sem
//! os rejeitados.

use crate::layouts::LayoutCandidate;
use crate::prefs::{Preferences, design_key};

/// Sugestões consideradas por lâmina: as que passaram pelo rodízio (spec VL, regra 6).
pub const WINDOW: usize = 12;

/// As sugestões de uma lâmina e quais são favoritas.
#[derive(Debug, Clone, Default)]
pub struct Window {
    pub list: Vec<LayoutCandidate>,
    pub favorite: Vec<bool>,
}

/// Recorta a lista da variedade. Sem preferências, são as `WINDOW` primeiras, como sempre.
pub fn window(list: Vec<LayoutCandidate>, aspects: &[f64], prefs: &Preferences) -> Window {
    if prefs.is_empty() {
        let list: Vec<_> = list.into_iter().take(WINDOW).collect();
        let favorite = vec![false; list.len()];
        return Window { list, favorite };
    }
    let keys: Vec<String> = list.iter().map(|c| design_key(c, aspects)).collect();
    // Todos rejeitados: vale a escolha sem preferências (regra 8).
    let all_rejected = keys.iter().all(|k| prefs.rejected.contains(k));
    let mut out = Window::default();
    let mut kept = 0;
    for (candidate, key) in list.into_iter().zip(&keys) {
        if !all_rejected && prefs.rejected.contains(key) {
            continue;
        }
        let favorite = prefs.favorites.contains(key);
        if kept < WINDOW || favorite {
            out.list.push(candidate);
            out.favorite.push(favorite);
        }
        kept += 1;
    }
    out
}
