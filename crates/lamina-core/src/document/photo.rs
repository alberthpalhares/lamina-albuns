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

/// Os ajustes da foto (spec AF): endireitar em décimos de grau (−45° a 45°), brilho e contraste
/// de −100 a 100 e preto e branco. São da foto, não do quadro: valem em todos os quadros dela
/// (regra 1). Neutro (tudo zero) não vai para o arquivo.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, JsonSchema)]
pub struct PhotoAdjust {
    /// Giro no sentido horário, em décimos de grau (o passo de 0,1° fica exato).
    #[serde(rename = "angulo_decimos", default)]
    pub angle_tenths: i32,
    #[serde(rename = "brilho", default)]
    pub brightness: i32,
    #[serde(rename = "contraste", default)]
    pub contrast: i32,
    #[serde(rename = "pb", default)]
    pub bw: bool,
}

impl PhotoAdjust {
    /// 45°, em décimos.
    pub const MAX_ANGLE_TENTHS: i32 = 450;
    /// O limite do brilho e do contraste, nos dois sentidos.
    pub const MAX_TONE: i32 = 100;

    /// Sem ajuste nenhum: a foto sai como hoje.
    pub fn is_neutral(&self) -> bool {
        *self == PhotoAdjust::default()
    }

    /// O giro em graus.
    pub fn degrees(&self) -> f64 {
        f64::from(self.angle_tenths) / 10.0
    }

    /// Mexe no tom (brilho, contraste ou preto e branco)?
    pub fn has_tone(&self) -> bool {
        self.brightness != 0 || self.contrast != 0 || self.bw
    }

    /// Dentro das faixas da spec (regra 9 e seção 6).
    pub fn is_valid(&self) -> bool {
        self.angle_tenths.abs() <= Self::MAX_ANGLE_TENTHS
            && self.brightness.abs() <= Self::MAX_TONE
            && self.contrast.abs() <= Self::MAX_TONE
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
    /// Endireitar, brilho, contraste e preto e branco (spec AF; formato 5). Neutro não é gravado.
    #[serde(default, skip_serializing_if = "PhotoAdjust::is_neutral")]
    pub adjust: PhotoAdjust,
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
            adjust: PhotoAdjust::default(),
        };
        assert_eq!(photo.orientation(), Orientation::Landscape);
        assert!((photo.aspect_ratio() - 1.5).abs() < 1e-12);
    }
}
