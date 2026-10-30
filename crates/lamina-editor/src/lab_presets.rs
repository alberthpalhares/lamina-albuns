//! Presets de laboratório (spec PR, regra 1; decisão do dono de 2026-10-29). Os números vêm do
//! próprio laboratório, nunca do software de referência (regra 6 do AGENTS).

use lamina_core::{ExportPrefs, PageLimits};

use crate::album::SpecFields;
use crate::presets::Preset;

/// A encadernadora dos presets abaixo, no filtro da etapa 1 (spec ET).
pub const PROALBUNS: &str = "ProAlbuns";

/// Os álbuns da ProAlbuns. Ela escreve altura × largura ("15x20"); o Lamina escreve, como em todos,
/// largura × altura da página ("20x15cm", correção do dono de 2026-10-29), e o 20x20 dela saiu. O
/// arquivo que ela pede é a lâmina aberta com a sangria só em cima e embaixo: 15,2 × 40 cm no
/// 20x15cm, então a página tem 20 × 15 cm e 1 mm de sangria em cima e embaixo, nada nas laterais.
pub fn proalbuns_presets() -> Vec<Preset> {
    // (largura da página, altura sem sangria, sangria em cima e embaixo em mm)
    [
        (20.0, 15.0, 1.0),
        (15.0, 20.0, 1.5),
        (25.0, 20.0, 1.5),
        (20.0, 25.0, 2.0),
    ]
    .into_iter()
    .map(|(width_cm, height_cm, top)| Preset {
        name: crate::preset_names::size_name(width_cm, height_cm),
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
        binder: Some(PROALBUNS.to_owned()),
        album_type: None,
    })
    .collect()
}
