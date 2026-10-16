//! Unidades físicas.
//!
//! Todo o documento é medido em **milímetros** ([`Mm`]). Pixels só aparecem na hora de
//! renderizar ou exportar, a partir do [`Dpi`] da especificação do álbum.

use std::ops::{Add, Div, Mul, Sub};

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::CoreError;

/// Milímetros em uma polegada.
pub const MM_PER_INCH: f64 = 25.4;

/// Um comprimento em milímetros.
#[derive(
    Debug, Clone, Copy, PartialEq, PartialOrd, Default, Serialize, Deserialize, JsonSchema,
)]
#[serde(transparent)]
pub struct Mm(pub f64);

impl Mm {
    pub const ZERO: Mm = Mm(0.0);

    pub fn from_cm(cm: f64) -> Self {
        Mm(cm * 10.0)
    }

    pub fn from_inches(inches: f64) -> Self {
        Mm(inches * MM_PER_INCH)
    }

    pub fn from_px(px: u32, dpi: Dpi) -> Self {
        Mm::from_inches(f64::from(px) / f64::from(dpi.get()))
    }

    pub fn get(self) -> f64 {
        self.0
    }

    pub fn to_inches(self) -> f64 {
        self.0 / MM_PER_INCH
    }

    /// Converte para pixels, arredondando para o inteiro mais próximo.
    ///
    /// Medidas negativas ou `NaN` viram 0 pixel.
    // `as` de f64 para u32 satura: negativos e NaN viram 0, valores enormes viram u32::MAX.
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    pub fn to_px(self, dpi: Dpi) -> u32 {
        (self.to_inches() * f64::from(dpi.get())).round() as u32
    }

    pub fn is_finite_non_negative(self) -> bool {
        self.0.is_finite() && self.0 >= 0.0
    }

    pub fn is_finite_positive(self) -> bool {
        self.0.is_finite() && self.0 > 0.0
    }
}

impl Add for Mm {
    type Output = Mm;
    fn add(self, rhs: Mm) -> Mm {
        Mm(self.0 + rhs.0)
    }
}

impl Sub for Mm {
    type Output = Mm;
    fn sub(self, rhs: Mm) -> Mm {
        Mm(self.0 - rhs.0)
    }
}

impl Mul<f64> for Mm {
    type Output = Mm;
    fn mul(self, rhs: f64) -> Mm {
        Mm(self.0 * rhs)
    }
}

impl Div<f64> for Mm {
    type Output = Mm;
    fn div(self, rhs: f64) -> Mm {
        Mm(self.0 / rhs)
    }
}

/// Resolução de impressão em pontos por polegada. Sempre entre 1 e [`Dpi::MAX`].
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, JsonSchema,
)]
#[serde(try_from = "u32", into = "u32")]
pub struct Dpi(u32);

impl Dpi {
    /// Limite superior generoso: nenhum laboratório imprime fotos acima disso.
    pub const MAX: u32 = 2400;

    pub fn new(value: u32) -> Result<Self, CoreError> {
        if (1..=Self::MAX).contains(&value) {
            Ok(Dpi(value))
        } else {
            Err(CoreError::InvalidDpi(value))
        }
    }

    pub fn get(self) -> u32 {
        self.0
    }
}

impl TryFrom<u32> for Dpi {
    type Error = CoreError;
    fn try_from(value: u32) -> Result<Self, Self::Error> {
        Dpi::new(value)
    }
}

impl From<Dpi> for u32 {
    fn from(dpi: Dpi) -> u32 {
        dpi.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dpi(v: u32) -> Dpi {
        Dpi::new(v).unwrap()
    }

    #[test]
    fn uma_polegada_a_300_dpi_tem_300_px() {
        assert_eq!(Mm(25.4).to_px(dpi(300)), 300);
    }

    #[test]
    fn trinta_centimetros_a_300_dpi() {
        // 300 mm / 25,4 × 300 = 3543,3 → 3543
        assert_eq!(Mm::from_cm(30.0).to_px(dpi(300)), 3543);
    }

    #[test]
    fn ida_e_volta_px_mm_px() {
        for px in [1, 300, 3543, 7087, 12_000] {
            assert_eq!(Mm::from_px(px, dpi(300)).to_px(dpi(300)), px);
        }
    }

    #[test]
    fn medidas_negativas_viram_zero_pixel() {
        assert_eq!(Mm(-10.0).to_px(dpi(300)), 0);
        assert_eq!(Mm(f64::NAN).to_px(dpi(300)), 0);
    }

    #[test]
    fn dpi_fora_da_faixa_e_rejeitado() {
        assert!(matches!(Dpi::new(0), Err(CoreError::InvalidDpi(0))));
        let too_big = Dpi::MAX + 1;
        assert!(matches!(Dpi::new(too_big), Err(CoreError::InvalidDpi(v)) if v == too_big));
        assert_eq!(Dpi::new(300).unwrap().get(), 300);
    }

    #[test]
    fn dpi_invalido_no_json_e_rejeitado() {
        let err = serde_json::from_str::<Dpi>("0").unwrap_err();
        assert!(err.to_string().contains("DPI inválido"), "{err}");
        assert_eq!(serde_json::from_str::<Dpi>("240").unwrap().get(), 240);
    }
}
