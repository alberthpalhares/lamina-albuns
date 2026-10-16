//! Fotos vinculadas ao projeto.

use std::path::PathBuf;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::PhotoId;
use crate::CaptureTime;

/// Orientação de uma foto ou quadro.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Orientation {
    Landscape,
    Portrait,
    Square,
}

impl Orientation {
    /// Proporções até 2% de diferença contam como quadradas.
    pub fn from_aspect_ratio(ratio: f64) -> Self {
        if (ratio - 1.0).abs() <= 0.02 {
            Orientation::Square
        } else if ratio > 1.0 {
            Orientation::Landscape
        } else {
            Orientation::Portrait
        }
    }
}

/// Uma foto vinculada ao projeto.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Photo {
    pub id: PhotoId,
    /// Caminho relativo à pasta do arquivo de projeto.
    pub path: PathBuf,
    /// Tamanho em pixels, já com a rotação EXIF aplicada.
    pub width_px: u32,
    pub height_px: u32,
    /// Impressão digital do conteúdo, usada para reencontrar o arquivo (preenchida pelo lamina-media).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fingerprint: Option<String>,
    /// Data e hora da captura (EXIF), usada para ordenar e agrupar por momento.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub taken_at: Option<CaptureTime>,
    /// Modelo da câmera (EXIF), útil para sincronizar horários entre câmeras.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub camera: Option<String>,
}

impl Photo {
    pub fn aspect_ratio(&self) -> f64 {
        f64::from(self.width_px) / f64::from(self.height_px.max(1))
    }

    pub fn orientation(&self) -> Orientation {
        Orientation::from_aspect_ratio(self.aspect_ratio())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn orientacao_pela_proporcao() {
        assert_eq!(Orientation::from_aspect_ratio(1.5), Orientation::Landscape);
        assert_eq!(Orientation::from_aspect_ratio(0.66), Orientation::Portrait);
        assert_eq!(Orientation::from_aspect_ratio(1.01), Orientation::Square);
    }

    #[test]
    fn foto_6000x4000_e_horizontal() {
        let photo = Photo {
            id: PhotoId(1),
            path: PathBuf::from("a.jpg"),
            width_px: 6000,
            height_px: 4000,
            fingerprint: None,
            taken_at: None,
            camera: None,
        };
        assert_eq!(photo.orientation(), Orientation::Landscape);
        assert!((photo.aspect_ratio() - 1.5).abs() < 1e-12);
    }
}
