//! Avisos dos quadros de texto (spec TX, seção 4): não cabe, fora da zona segura (ou na dobra),
//! caractere que a fonte não tem e fonte que não está neste computador (spec FS).

use lamina_core::{PageSide, Project, Rect, TextBox};
use lamina_render::check_text;

use crate::preflight::WarningKind;

/// Quantos caracteres do texto aparecem no aviso.
const EXCERPT_CHARS: usize = 20;

/// O começo do texto para o aviso: a 1ª linha, até 20 caracteres, com "…" se cortou algo.
pub(crate) fn excerpt(text: &str) -> String {
    let trimmed = text.trim();
    let first = trimmed.lines().next().unwrap_or_default();
    let short: String = first.chars().take(EXCERPT_CHARS).collect();
    if short.len() < trimmed.len() {
        format!("{}…", short.trim_end())
    } else {
        short
    }
}

/// Os avisos de um texto no quadro `rect` (em mm). Texto vazio não sai, então não avisa.
pub(crate) fn text_warnings(project: &Project, text: &TextBox, rect: Rect) -> Vec<WarningKind> {
    if text.is_blank() {
        return Vec::new();
    }
    let what = excerpt(&text.text);
    let check = check_text(text, (rect.width.get(), rect.height.get()));
    let mut out = Vec::new();
    if check.overflow {
        out.push(WarningKind::TextOverflow { text: what.clone() });
    }
    if !inside_safe(project, rect) {
        out.push(WarningKind::TextOutsideSafe { text: what });
    }
    out.extend(
        check
            .missing
            .into_iter()
            .map(|c| WarningKind::MissingChar { char: c }),
    );
    if check.unknown_font {
        out.push(WarningKind::UnknownFont {
            font: text.font.clone(),
        });
    }
    out
}

/// O quadro cabe inteiro na zona segura de uma das páginas; atravessar a dobra não cabe.
fn inside_safe(project: &Project, rect: Rect) -> bool {
    [PageSide::Left, PageSide::Right]
        .iter()
        .any(|side| project.spec.safe_rect(*side).contains_rect(&rect))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trecho_corta_na_primeira_linha_ou_em_vinte_caracteres() {
        assert_eq!(excerpt("Ana & Pedro"), "Ana & Pedro");
        assert_eq!(excerpt("Ana & Pedro\n2026"), "Ana & Pedro…");
        assert_eq!(
            excerpt("Um texto bem comprido que passa de vinte"),
            "Um texto bem comprid…"
        );
    }
}
