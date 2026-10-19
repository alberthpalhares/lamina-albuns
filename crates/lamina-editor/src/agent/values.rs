//! Valores que a IA escreve como texto ou número: cores (hex ou nome da paleta), fontes e as
//! faixas da janela, com as recusas em português.

use lamina_core::Rgb;
use lamina_render::FAMILIES;

use super::ToolError;
use crate::{PALETTE, parse_color};

/// "#F5F0E6" ou um nome da paleta da aba Aparência ("Marfim"), sem diferença de maiúsculas.
pub(super) fn color(text: &str) -> Result<Rgb, ToolError> {
    let name = text.trim();
    parse_color(name)
        .or_else(|| {
            PALETTE
                .iter()
                .find(|(n, _)| n.to_lowercase() == name.to_lowercase())
                .map(|(_, c)| *c)
        })
        .ok_or_else(|| ToolError::BadColor {
            text: text.to_owned(),
            palette: PALETTE.map(|(n, _)| n).join(", "),
        })
}

/// O nome de uma das fontes do Lamina, como está na lista (sem diferença de maiúsculas).
pub(super) fn font(name: &str) -> Result<&'static str, ToolError> {
    FAMILIES
        .iter()
        .find(|f| f.name.eq_ignore_ascii_case(name.trim()))
        .map(|f| f.name)
        .ok_or_else(|| ToolError::NoFont {
            name: name.to_owned(),
            known: FAMILIES.map(|f| f.name).join(", "),
        })
}

/// `value` dentro de `range` (inclusive); senão, a recusa `why`.
pub(super) fn within(value: f64, range: (f64, f64), why: &'static str) -> Result<f64, ToolError> {
    if value.is_finite() && (range.0..=range.1).contains(&value) {
        Ok(value)
    } else {
        Err(ToolError::OutOfRange(why))
    }
}
