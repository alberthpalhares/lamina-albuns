//! O medidor de páginas da barra de cima (spec DJ, regra 13) e o selo das pontas com página simples
//! na faixa de lâminas (regra 17).

use lamina_core::{Edges, PageLimits};

/// O que o medidor mostra.
#[derive(Debug, Clone, PartialEq)]
pub struct PageMeter {
    /// "mínimo 10 · máximo 120 páginas".
    pub text: String,
    /// "10 a 120 páginas", para janela estreita (spec DJ, seção 6).
    pub short: String,
}

fn pages_word(pages: u32) -> &'static str {
    if pages == 1 { "página" } else { "páginas" }
}

/// O medidor de um álbum com estes limites; sem limites, não há medidor.
pub fn page_meter(limits: PageLimits) -> Option<PageMeter> {
    let (text, short) = match (limits.min, limits.max) {
        (None, None) => return None,
        (Some(min), None) => {
            let text = format!("mínimo {min} {}", pages_word(min));
            (text.clone(), text)
        }
        (None, Some(max)) => {
            let text = format!("máximo {max} {}", pages_word(max));
            (text.clone(), text)
        }
        (Some(min), Some(max)) => {
            let word = pages_word(max);
            (
                format!("mínimo {min} · máximo {max} {word}"),
                format!("{min} a {max} {word}"),
            )
        }
    };
    Some(PageMeter { text, short })
}

/// O selo da miniatura: só as pontas com página simples dizem "1 pág."; o total já está no resumo.
pub fn page_badge(edges: Edges, index: usize, spreads: usize) -> &'static str {
    let active = edges.active_pages(index, spreads);
    if active.left && active.right {
        ""
    } else {
        "1 pág."
    }
}
