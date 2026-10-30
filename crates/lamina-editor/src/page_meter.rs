//! O medidor de páginas da barra de cima (spec DJ, regra 13) e o selo das pontas com página simples
//! na faixa de lâminas (regra 17).

use lamina_core::{Edges, PageLimits};

/// O que o medidor mostra.
#[derive(Debug, Clone, PartialEq)]
pub struct PageMeter {
    /// "12 páginas · mínimo 10 · máximo 120".
    pub text: String,
    /// "12 de 120 páginas", para janela estreita (spec DJ, seção 6).
    pub short: String,
    /// A parte usada da barra, de 0 a 1.
    pub used: f32,
    /// Onde fica o tique do mínimo, de 0 a 1, se houver mínimo.
    pub tick: Option<f32>,
    /// Dentro da faixa de páginas.
    pub inside: bool,
}

fn pages_text(pages: usize) -> String {
    if pages == 1 {
        "1 página".to_owned()
    } else {
        format!("{pages} páginas")
    }
}

/// O medidor de um álbum de `pages` páginas; sem limites, não há medidor.
#[allow(clippy::cast_precision_loss)]
pub fn page_meter(pages: usize, limits: PageLimits) -> Option<PageMeter> {
    if limits.is_empty() {
        return None;
    }
    let mut text = pages_text(pages);
    if let Some(min) = limits.min {
        text.push_str(&format!(" · mínimo {min}"));
    }
    if let Some(max) = limits.max {
        text.push_str(&format!(" · máximo {max}"));
    }
    let short = limits
        .max
        .map_or_else(|| text.clone(), |max| format!("{pages} de {max} páginas"));
    // A barra vai até o máximo; só com mínimo, até o dobro dele.
    let scale = limits
        .max
        .or(limits.min.map(|min| min * 2))
        .unwrap_or(1)
        .max(1) as f32;
    Some(PageMeter {
        text,
        short,
        used: (pages as f32 / scale).clamp(0.0, 1.0),
        tick: limits.min.map(|min| (min as f32 / scale).clamp(0.0, 1.0)),
        inside: limits.warning(pages).is_none(),
    })
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
