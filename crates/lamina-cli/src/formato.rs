//! `--formato` e `--marcas-de-corte` do `lamina build` e do `lamina exportar` (spec PD, seção 3).

use anyhow::{Result, bail};
use clap::{Args, ValueEnum};
use lamina_export::ExportFormat;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, ValueEnum)]
pub enum Formato {
    /// Um JPEG por lâmina (ou por página).
    #[default]
    Jpeg,
    /// Um PDF/X-4 do álbum inteiro.
    Pdf,
}

#[derive(Debug, Args)]
pub struct SaidaArgs {
    /// Formato da exportação.
    #[arg(long, value_enum, default_value_t = Formato::Jpeg)]
    pub formato: Formato,
    /// Marcas de corte nos cantos (só no PDF).
    #[arg(long)]
    pub marcas_de_corte: bool,
}

impl SaidaArgs {
    /// O formato e as marcas, ou o erro da seção 6 da spec.
    pub fn options(&self) -> Result<(ExportFormat, bool)> {
        match self.formato {
            Formato::Jpeg if self.marcas_de_corte => {
                bail!("marcas de corte só existem no PDF (use --formato pdf).")
            }
            Formato::Jpeg => Ok((ExportFormat::Jpeg, false)),
            Formato::Pdf => Ok((ExportFormat::Pdf, self.marcas_de_corte)),
        }
    }
}
