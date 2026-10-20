//! Comando `lamina spec`: medidas de impressão de uma especificação de álbum.

use anyhow::{Result, bail};
use clap::Args;
use lamina_core::{AlbumEdge, AlbumSpec, ExportPrefs, PageLimits, PageSide};
use lamina_editor::{PRESETS_FILE, Preset, PresetsStore, SpecFields, config_dir};

/// Medidas do álbum, comuns a `lamina spec` e `lamina build`.
#[derive(Debug, Args)]
pub struct AlbumArgs {
    /// Preset com as medidas, a faixa de páginas e a exportação (veja `lamina presets`); as opções
    /// abaixo valem por cima dele.
    #[arg(long)]
    pub preset: Option<String>,
    /// Largura da página, em centímetros (padrão 30).
    #[arg(long)]
    pub largura_cm: Option<f64>,
    /// Altura da página, em centímetros (padrão 30).
    #[arg(long)]
    pub altura_cm: Option<f64>,
    /// Resolução de impressão (padrão 300).
    #[arg(long)]
    pub dpi: Option<u32>,
    /// Sangria em volta da lâmina, em milímetros (padrão 3).
    #[arg(long)]
    pub sangria_mm: Option<f64>,
    /// Zona segura nas bordas externas e de cima/baixo, em milímetros (padrão 10).
    #[arg(long)]
    pub zona_segura_mm: Option<f64>,
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

/// Os presets do Lamina e os do fotógrafo (o mesmo arquivo da janela).
pub fn presets() -> PresetsStore {
    PresetsStore::load(&config_dir().join(PRESETS_FILE))
}

impl AlbumArgs {
    /// O preset pedido, se houver (spec PR, seção 6).
    pub fn preset(&self) -> Result<Option<Preset>> {
        let Some(name) = &self.preset else {
            return Ok(None);
        };
        match presets().find(name) {
            Some(preset) => Ok(Some(preset)),
            None => bail!("o preset \"{name}\" não existe. Veja a lista com lamina presets."),
        }
    }

    /// Os campos: os do preset (ou os padrões), com as opções digitadas por cima.
    pub fn fields(&self, preset: Option<&Preset>) -> SpecFields {
        let base = preset.map_or_else(SpecFields::default, |p| p.fields.clone());
        SpecFields {
            width_cm: self.largura_cm.unwrap_or(base.width_cm),
            height_cm: self.altura_cm.unwrap_or(base.height_cm),
            dpi: self.dpi.unwrap_or(base.dpi),
            bleed_mm: self.sangria_mm.unwrap_or(base.bleed_mm),
            safe_mm: self.zona_segura_mm.unwrap_or(base.safe_mm),
            spine_mm: self.lombada_mm.or(base.spine_mm),
            starts_single: self.inicia_com_pagina || base.starts_single,
            ends_single: self.termina_com_pagina || base.ends_single,
            ..base
        }
    }

    /// As mesmas contas da janela "Novo álbum" (`SpecFields`).
    pub fn to_spec(&self) -> Result<AlbumSpec> {
        let preset = self.preset()?;
        Ok(self.fields(preset.as_ref()).to_spec()?)
    }
}

/// A faixa de páginas e a exportação do preset (ou as de sempre).
pub fn preset_extras(preset: Option<&Preset>) -> (PageLimits, ExportPrefs) {
    preset.map_or_else(Default::default, |p| (p.limits, p.export))
}

/// `lamina presets`: uma linha por preset.
pub fn print_presets() {
    for p in presets().all() {
        let f = &p.fields;
        let pages = match (p.limits.min, p.limits.max) {
            (Some(a), Some(b)) => format!("{a} a {b} páginas"),
            (Some(a), None) => format!("a partir de {a} páginas"),
            (None, Some(b)) => format!("até {b} páginas"),
            (None, None) => "páginas livres".to_owned(),
        };
        println!(
            "{} — {} × {} cm, {} DPI, sangria {} mm, {pages}",
            p.name, f.width_cm, f.height_cm, f.dpi, f.bleed_mm
        );
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
