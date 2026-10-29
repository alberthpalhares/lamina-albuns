//! Valores que a IA escreve como texto ou número: cores (hex ou nome da paleta), fontes e as
//! faixas da janela, com as recusas em português.

use lamina_core::Rgb;
use lamina_render::{FAMILIES, installed_name, installed_names};

use crate::font_picker::{fold, system_fonts_allowed};

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

/// O nome certo de uma fonte, sem diferença de maiúsculas: uma das 6 do Lamina ou, com a trava
/// aberta, uma do computador (spec FS, regra 8).
pub(super) fn font(name: &str) -> Result<String, ToolError> {
    let wanted = name.trim();
    if let Some(f) = FAMILIES
        .iter()
        .find(|f| f.name.eq_ignore_ascii_case(wanted))
    {
        return Ok(f.name.to_owned());
    }
    if system_fonts_allowed()
        && let Some(found) = installed_name(wanted)
    {
        return Ok(found);
    }
    Err(ToolError::NoFont {
        name: name.to_owned(),
        known: known_fonts(wanted),
    })
}

/// As 6 do Lamina e, com a trava aberta, até 5 do computador com um trecho do nome pedido.
fn known_fonts(wanted: &str) -> String {
    let lamina = FAMILIES.map(|f| f.name).join(", ");
    if !system_fonts_allowed() {
        return lamina;
    }
    let names = installed_names();
    let words: Vec<String> = wanted
        .split_whitespace()
        .map(fold)
        .filter(|w| w.chars().count() >= 3)
        .collect();
    let like: Vec<&str> = names
        .iter()
        .filter(|n| words.iter().any(|w| fold(n).contains(w.as_str())))
        .take(5)
        .map(String::as_str)
        .collect();
    if like.is_empty() {
        format!("{lamina}, ou o nome exato de uma fonte deste computador")
    } else {
        format!(
            "{lamina}, ou uma fonte deste computador, como {}",
            like.join(", ")
        )
    }
}

/// `value` dentro de `range` (inclusive); senão, a recusa `why`.
pub(super) fn within(value: f64, range: (f64, f64), why: &'static str) -> Result<f64, ToolError> {
    if value.is_finite() && (range.0..=range.1).contains(&value) {
        Ok(value)
    } else {
        Err(ToolError::OutOfRange(why))
    }
}
