//! Comando `lamina spec`: medidas de impressão de uma especificação de álbum.

use anyhow::Result;
use clap::Args;
use lamina_core::{AlbumEdge, AlbumSpec, PageSide};
use lamina_editor::SpecFields;

/// Medidas do álbum, comuns a `lamina spec` e `lamina build`.
#[derive(Debug, Args)]
pub struct AlbumArgs {
    /// Largura da página, em centímetros.
    #[arg(long, default_value_t = 30.0)]
    pub largura_cm: f64,
    /// Altura da página, em centímetros.
    #[arg(long, default_value_t = 30.0)]
    pub altura_cm: f64,
    /// Resolução de impressão.
    #[arg(long, default_value_t = 300)]
    pub dpi: u32,
    /// Sangria em volta da lâmina, em milímetros.
    #[arg(long, default_value_t = 3.0)]
    pub sangria_mm: f64,
    /// Zona segura nas bordas externas e de cima/baixo, em milímetros.
    #[arg(long, default_value_t = 10.0)]
    pub zona_segura_mm: f64,
    /// Zona segura do lado da lombada, em milímetros (padrão: igual à zona segura).
    #[arg(long)]
    pub lombada_mm: Option<f64>,
    /// O miolo começa com uma página simples (só a direita).
    #[arg(long)]
    pub inicia_com_pagina: bool,
    /// O miolo termina com uma página simples (só a esquerda).
    #[arg(long)]
    pub termina_com_pagina: bool,
}

/// `lamina spec`: as medidas do álbum e o número de lâminas para contar as páginas.
#[derive(Debug, Args)]
pub struct SpecArgs {
    #[command(flatten)]
    pub album: AlbumArgs,
    /// Número de lâminas, para calcular o total de páginas.
    #[arg(long, default_value_t = 10)]
    pub laminas: usize,
}

pub fn edge(single: bool) -> AlbumEdge {
    if single {
        AlbumEdge::SinglePage
    } else {
        AlbumEdge::FullSpread
    }
}

impl AlbumArgs {
    /// As mesmas contas da janela "Novo álbum" (`SpecFields`).
    pub fn to_spec(&self) -> Result<AlbumSpec> {
        let fields = SpecFields {
            width_cm: self.largura_cm,
            height_cm: self.altura_cm,
            dpi: self.dpi,
            bleed_mm: self.sangria_mm,
            safe_mm: self.zona_segura_mm,
            spine_mm: self.lombada_mm,
            starts_single: self.inicia_com_pagina,
            ends_single: self.termina_com_pagina,
        };
        Ok(fields.to_spec()?)
    }
}

pub fn print_spec(spec: &AlbumSpec, spreads: usize) {
    let trim = spec.trim_rect();
    let canvas = spec.canvas_rect();
    let safe = spec.safe_rect(PageSide::Left);
    let (trim_w, trim_h) = spec.trim_size_px();
    let (canvas_w, canvas_h) = spec.canvas_size_px();
    println!(
        "Lâmina aberta (corte final): {:.1} × {:.1} mm",
        trim.width.get(),
        trim.height.get()
    );
    println!(
        "Com sangria (arquivo):       {:.1} × {:.1} mm",
        canvas.width.get(),
        canvas.height.get()
    );
    println!(
        "Zona segura por página:      {:.1} × {:.1} mm",
        safe.width.get(),
        safe.height.get()
    );
    println!(
        "Pixels a {} DPI: corte {trim_w} × {trim_h}, arquivo {canvas_w} × {canvas_h}",
        spec.dpi().get()
    );
    println!(
        "{spreads} lâminas = {} páginas impressas",
        spec.edges().page_count(spreads)
    );
}
