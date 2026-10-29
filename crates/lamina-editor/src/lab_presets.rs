//! Presets de laboratório (spec PR, regra 1; decisão do dono de 2026-10-29). Os números vêm do
//! próprio laboratório, nunca do software de referência (regra 6 do AGENTS).

use lamina_core::{ExportPrefs, PageLimits};

use crate::album::SpecFields;
use crate::presets::Preset;

/// O laboratório dos presets abaixo, no filtro da etapa 1 (spec ET).
pub const PROALBUNS: &str = "ProAlbuns";

/// Os álbuns da ProAlbuns, com o nome que o laboratório usa (altura × largura da página). O
/// arquivo que ela pede é a lâmina aberta com a sangria só em cima e embaixo: 15,2 × 40 cm no
/// 15x20, então a página tem 20 × 15 cm e 1 mm de sangria em cima e embaixo, nada nas laterais.
pub fn proalbuns_presets() -> Vec<Preset> {
    // (nome, largura da página, altura sem sangria, sangria em cima e embaixo em mm)
    [
        ("ProAlbuns 15x20", 20.0, 15.0, 1.0),
        ("ProAlbuns 20x15", 15.0, 20.0, 1.5),
        ("ProAlbuns 20x20", 20.0, 20.0, 1.5),
        ("ProAlbuns 20x25", 25.0, 20.0, 1.5),
        ("ProAlbuns 25x20", 20.0, 25.0, 2.0),
    ]
    .into_iter()
    .map(|(name, width_cm, height_cm, top)| Preset {
        name: name.to_owned(),
        fields: SpecFields {
            width_cm,
            height_cm,
            bleed_mm: 0.0,
            bleed_top_mm: Some(top),
            ..SpecFields::default()
        },
        limits: PageLimits {
            min: Some(20),
            max: Some(80),
        },
        export: ExportPrefs::default(),
        lab: Some(PROALBUNS.to_owned()),
    })
    .collect()
}
