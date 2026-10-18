//! O que o laboratório pede (spec PR): a faixa de páginas e as preferências de exportação do
//! projeto. Os dois campos são opcionais no arquivo: um projeto antigo abre igual (regra 9).

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::ColorSpace;

/// Mínimo e máximo de páginas do álbum (nenhum = sem limite).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, JsonSchema)]
pub struct PageLimits {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max: Option<u32>,
}

impl PageLimits {
    pub fn is_empty(&self) -> bool {
        self.min.is_none() && self.max.is_none()
    }

    /// "O álbum tem 18 páginas; o mínimo é 20." (ou o máximo), se `pages` sai da faixa.
    pub fn warning(&self, pages: usize) -> Option<String> {
        let n = u32::try_from(pages).unwrap_or(u32::MAX);
        if let Some(min) = self.min.filter(|min| n < *min) {
            return Some(format!("O álbum tem {pages} páginas; o mínimo é {min}."));
        }
        self.max
            .filter(|max| n > *max)
            .map(|max| format!("O álbum tem {pages} páginas; o máximo é {max}."))
    }
}

/// Tipo de arquivo da exportação.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum OutputFormat {
    #[default]
    Jpeg,
    Pdf,
    Tiff,
}

/// Como o laboratório quer os arquivos: formato, cor e páginas soltas.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, JsonSchema)]
pub struct ExportPrefs {
    #[serde(default)]
    pub format: OutputFormat,
    #[serde(default)]
    pub space: ColorSpace,
    #[serde(default)]
    pub pages: bool,
}

impl ExportPrefs {
    pub fn is_default(&self) -> bool {
        *self == ExportPrefs::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn aviso_so_fora_da_faixa() {
        let limits = PageLimits {
            min: Some(20),
            max: Some(80),
        };
        assert_eq!(
            limits.warning(18).as_deref(),
            Some("O álbum tem 18 páginas; o mínimo é 20.")
        );
        assert_eq!(
            limits.warning(84).as_deref(),
            Some("O álbum tem 84 páginas; o máximo é 80.")
        );
        assert_eq!(limits.warning(20), None);
        assert_eq!(PageLimits::default().warning(2), None);
    }
}
