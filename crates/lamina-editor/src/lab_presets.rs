//! Presets de laboratório (spec PR, regra 1; decisão do dono de 2026-10-29). Os números vêm do
//! próprio laboratório, nunca do software de referência (regra 6 do AGENTS).

use lamina_core::{ExportPrefs, PageLimits};

use crate::album::SpecFields;
use crate::presets::Preset;

/// A encadernadora dos presets abaixo, no filtro da etapa 1 (spec ET).
pub const PROALBUNS: &str = "ProAlbuns";

/// Os álbuns da ProAlbuns, da tabela que o dono dela mandou ao dono do Lamina em 2026-10-29. Ela
/// escreve altura × largura ("15x21"); o Lamina escreve largura × altura da página ("20x15cm"). O
/// arquivo que ela pede é a lâmina aberta com a sangria só em cima e embaixo: a largura é o dobro
/// da página, e a altura ganha 1 mm em cima e embaixo nas páginas de 15 cm, 1,5 mm nas de 20, 2 mm
/// nas de 25 e 2,5 mm nas de 30 (15,2; 20,3; 25,4 e 30,5 cm). Faltam o 35x35 e o vertical de
/// 30 × 40, que ela vai mandar.
pub fn proalbuns_presets() -> Vec<Preset> {
    // (largura da página, altura sem sangria), em cm: quadrados, horizontais e verticais.
    [
        (15.0, 15.0),
        (20.0, 20.0),
        (25.0, 25.0),
        (30.0, 30.0),
        (20.0, 15.0),
        (25.0, 20.0),
        (30.0, 20.0),
        (30.0, 25.0),
        (35.0, 30.0),
        (40.0, 30.0),
        (45.0, 30.0),
        (15.0, 20.0),
        (20.0, 25.0),
        (25.0, 30.0),
    ]
    .into_iter()
    .map(|(width_cm, height_cm)| Preset {
        name: crate::preset_names::size_name(width_cm, height_cm),
        fields: SpecFields {
            width_cm,
            height_cm,
            bleed_mm: 0.0,
            bleed_top_mm: Some(top_bleed_mm(height_cm)),
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

/// A sangria em cima e embaixo pela altura da página, como na tabela da ProAlbuns.
fn top_bleed_mm(height_cm: f64) -> f64 {
    match height_cm {
        h if h <= 15.0 => 1.0,
        h if h <= 20.0 => 1.5,
        h if h <= 25.0 => 2.0,
        _ => 2.5,
    }
}
