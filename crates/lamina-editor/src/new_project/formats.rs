//! Os três formatos da etapa 1 (spec NP, regra 1): quadrado, horizontal e vertical, pela
//! proporção da página, cada um com os seus tamanhos (presets do Lamina e do fotógrafo).

use crate::Preset;
use crate::album::SpecFields;

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Format {
    #[serde(rename = "quadrado")]
    Square,
    #[serde(rename = "horizontal")]
    Landscape,
    #[serde(rename = "vertical")]
    Portrait,
}

impl Format {
    pub const ALL: [Format; 3] = [Format::Square, Format::Landscape, Format::Portrait];

    /// Quadrado se largura = altura; horizontal se a largura é maior; vertical se é menor.
    pub fn of(fields: &SpecFields) -> Format {
        let (w, h) = (fields.width_cm, fields.height_cm);
        if (w - h).abs() < 1e-9 {
            Format::Square
        } else if w > h {
            Format::Landscape
        } else {
            Format::Portrait
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Format::Square => "Quadrado",
            Format::Landscape => "Horizontal",
            Format::Portrait => "Vertical",
        }
    }
}

/// Os tamanhos de cada formato, na ordem dos presets (os do Lamina, depois os do fotógrafo).
pub fn formats(presets: &[Preset]) -> Vec<(Format, Vec<Preset>)> {
    Format::ALL
        .into_iter()
        .map(|format| {
            let sizes = presets
                .iter()
                .filter(|p| Format::of(&p.fields) == format)
                .cloned()
                .collect();
            (format, sizes)
        })
        .collect()
}
