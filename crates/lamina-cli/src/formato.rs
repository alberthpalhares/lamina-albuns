//! `--formato`, `--marcas-de-corte` e `--cor` do `lamina build` e do `lamina exportar` (specs PD
//! e CT, seção 3).

use anyhow::{Result, bail};
use clap::{Args, ValueEnum};
use lamina_core::ColorSpace;
use lamina_export::ExportFormat;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, ValueEnum)]
pub enum Formato {
    /// Um JPEG por lâmina (ou por página).
    #[default]
    Jpeg,
    /// Um PDF/X-4 do álbum inteiro.
    Pdf,
    /// Um TIFF sem perda por lâmina (ou por página).
    Tiff,
}

/// Espaço de cor da saída (spec CT).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, ValueEnum)]
pub enum Cor {
    #[default]
    Srgb,
    /// Compatível com Adobe RGB (1998).
    AdobeRgb,
}

#[derive(Debug, Args)]
pub struct SaidaArgs {
    /// Formato da exportação.
    #[arg(long, value_enum, default_value_t = Formato::Jpeg)]
    pub formato: Formato,
    /// Marcas de corte nos cantos (só no PDF).
    #[arg(long)]
    pub marcas_de_corte: bool,
    /// Espaço de cor da saída.
    #[arg(long, value_enum, default_value_t = Cor::Srgb)]
    pub cor: Cor,
}

impl SaidaArgs {
    /// O formato e as marcas, ou o erro da seção 6 da spec.
    pub fn options(&self) -> Result<(ExportFormat, bool)> {
        match self.formato {
            Formato::Jpeg | Formato::Tiff if self.marcas_de_corte => {
                bail!("marcas de corte só existem no PDF (use --formato pdf).")
            }
            Formato::Jpeg => Ok((ExportFormat::Jpeg, false)),
            Formato::Tiff => Ok((ExportFormat::Tiff, false)),
            Formato::Pdf => Ok((ExportFormat::Pdf, self.marcas_de_corte)),
        }
    }

    pub fn space(&self) -> ColorSpace {
        match self.cor {
            Cor::Srgb => ColorSpace::Srgb,
            Cor::AdobeRgb => ColorSpace::AdobeRgb,
        }
    }
}
