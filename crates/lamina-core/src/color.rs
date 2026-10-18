//! Espaço de cor da saída (spec CT): sRGB (padrão) ou Adobe RGB (1998). As cores sólidas do
//! projeto (fundo, borda) são escolhidas em sRGB; numa saída em Adobe RGB são convertidas aqui,
//! com as fórmulas dos dois espaços (mesmo branco D65, nenhuma adaptação de branco).

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::Rgb;

/// Espaço de cor dos arquivos exportados.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ColorSpace {
    #[default]
    Srgb,
    /// Compatível com Adobe RGB (1998).
    AdobeRgb,
}

/// sRGB linear → XYZ (D65), IEC 61966-2-1.
const SRGB_TO_XYZ: [[f64; 3]; 3] = [
    [0.412_456_4, 0.357_576_1, 0.180_437_5],
    [0.212_672_9, 0.715_152_2, 0.072_175_0],
    [0.019_333_9, 0.119_192_0, 0.950_304_1],
];

/// XYZ (D65) → Adobe RGB (1998) linear.
const XYZ_TO_ADOBE: [[f64; 3]; 3] = [
    [2.041_369_0, -0.564_946_4, -0.344_694_4],
    [-0.969_266_0, 1.876_010_8, 0.041_556_0],
    [0.013_447_4, -0.118_389_7, 1.015_409_6],
];

/// Gama do Adobe RGB (1998): 563/256.
const ADOBE_GAMMA: f64 = 563.0 / 256.0;

impl ColorSpace {
    /// A cor sRGB `c` neste espaço. Em sRGB, a própria cor (nada muda: spec CT, regra 7).
    pub fn from_srgb(self, c: Rgb) -> Rgb {
        match self {
            ColorSpace::Srgb => c,
            ColorSpace::AdobeRgb => {
                let linear = [c.r, c.g, c.b].map(|v| srgb_decode(f64::from(v) / 255.0));
                let xyz = mul(&SRGB_TO_XYZ, linear);
                let [r, g, b] = mul(&XYZ_TO_ADOBE, xyz).map(|l| to_u8(adobe_encode(l)));
                Rgb { r, g, b }
            }
        }
    }
}

fn srgb_decode(v: f64) -> f64 {
    if v <= 0.040_45 {
        v / 12.92
    } else {
        ((v + 0.055) / 1.055).powf(2.4)
    }
}

fn adobe_encode(l: f64) -> f64 {
    l.clamp(0.0, 1.0).powf(1.0 / ADOBE_GAMMA)
}

fn mul(m: &[[f64; 3]; 3], v: [f64; 3]) -> [f64; 3] {
    [0, 1, 2].map(|r| m[r][0] * v[0] + m[r][1] * v[1] + m[r][2] * v[2])
}

fn to_u8(v: f64) -> u8 {
    // Preso entre 0 e 255 antes do cast.
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let n = (v * 255.0).round().clamp(0.0, 255.0) as u8;
    n
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn srgb_nao_muda_nada() {
        let c = Rgb {
            r: 12,
            g: 200,
            b: 77,
        };
        assert_eq!(ColorSpace::Srgb.from_srgb(c), c);
    }

    #[test]
    fn vermelho_srgb_em_adobe_rgb() {
        let red = ColorSpace::AdobeRgb.from_srgb(Rgb { r: 255, g: 0, b: 0 });
        assert!((i32::from(red.r) - 219).abs() <= 1, "{red:?}");
        assert!(red.g <= 1 && red.b <= 1, "{red:?}");
        // Branco, preto e cinzas neutros continuam neutros.
        for v in [0, 128, 255] {
            let gray = ColorSpace::AdobeRgb.from_srgb(Rgb { r: v, g: v, b: v });
            assert!(
                gray.r.abs_diff(gray.g) <= 1 && gray.g.abs_diff(gray.b) <= 1,
                "{gray:?}"
            );
        }
    }
}
