//! Textos do painel de layouts: o nome da família e a nota explicada (spec TL, seção 4).

use lamina_core::Project;
use lamina_layout::{FamilyKind, Score};

/// Aviso do painel quando todas as opções estão rejeitadas (spec TL, regra 7).
pub const ALL_REJECTED: &str = "Todas as opções desta seleção foram rejeitadas.";

/// Nome da família em português.
pub fn family_name(kind: FamilyKind) -> &'static str {
    match kind {
        FamilyKind::Across => "Atravessada",
        FamilyKind::PerPage => "Por página",
        FamilyKind::WholeSpread => "Lâmina inteira",
        FamilyKind::Hero => "Destaque com apoio",
        FamilyKind::Mirrored => "Espelhado",
        FamilyKind::BleedPage => "Página sangrada",
        FamilyKind::Mosaic => "Mosaico",
    }
}

// As partes da nota ficam entre −15 e 50: cabem com folga em i64.
#[allow(clippy::cast_possible_truncation)]
fn points(v: f64) -> i64 {
    v.round() as i64
}

/// "Nota 82 = aproveitamento 45 + menor foto 22 + equilíbrio 20 − recorte 5". A nota mostrada é
/// a soma das partes arredondadas, para a conta fechar (TL-B10).
pub fn score_text(score: &Score) -> String {
    let (c, s, b) = (
        points(score.coverage),
        points(score.smallest),
        points(score.balance),
    );
    let crop = points(-score.crop);
    format!(
        "Nota {} = aproveitamento {c} + menor foto {s} + equilíbrio {b} − recorte {crop}",
        c + s + b - crop
    )
}

/// Ampliação a partir da qual o texto vira aviso (a mesma da pré-impressão).
const WARN_PERCENT: f64 = 120.0;

/// "DPI efetivo 225 · ampliada 133%" do quadro `frame` da lâmina `index`, e se é aviso
/// (spec RC, regra 7). `None` se o quadro não tem foto.
pub fn frame_dpi_text(project: &Project, index: usize, frame: usize) -> Option<(String, bool)> {
    let f = project.spreads.get(index)?.frames.get(frame)?;
    let dpi = project.effective_dpi(f).ok()??;
    let percent = f64::from(project.spec.dpi().get()) / dpi * 100.0;
    let text = format!(
        "DPI efetivo {} · ampliada {}%",
        points(dpi),
        points(percent)
    );
    Some((text, percent > WARN_PERCENT))
}
