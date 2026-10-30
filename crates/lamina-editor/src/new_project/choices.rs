//! A escolha do tamanho na etapa 1 (spec ET): o filtro por encadernadora, os tamanhos de cada
//! formato, o tamanho que já vem marcado e o resumo de uma linha.

use lamina_core::PageLimits;

use super::formats::{Format, formats};
use super::size::{SizeField, SizeForm};
use super::units::number;
use crate::Preset;
use crate::album::SpecFields;
use crate::font_picker::fold;

/// O primeiro item do filtro (regra 4).
pub const ALL_BINDERS: &str = "Todas as encadernadoras";
/// O convite do fim da lista de encadernadoras (regra 5).
pub const ADD_BINDER: &str = "Adicionar a minha encadernadora…";

/// A encadernadora limpa: sem espaços nas pontas; vazio vira nenhuma (seção 6).
pub fn clean_binder(binder: Option<&str>) -> Option<String> {
    binder
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .map(str::to_owned)
}

/// As encadernadoras dos presets, sem repetir, em ordem alfabética sem ligar para maiúsculas e
/// acentos (correção de 2026-10-29, ET-B11).
pub fn binders(presets: &[Preset]) -> Vec<String> {
    let mut found: Vec<String> = Vec::new();
    for binder in presets
        .iter()
        .filter_map(|p| clean_binder(p.binder.as_deref()))
    {
        if !found.contains(&binder) {
            found.push(binder);
        }
    }
    found.sort_by_key(|b| fold(b));
    found
}

/// A lista que a janela mostra: "Todas as encadernadoras", as que batem com a busca (sem ligar
/// para maiúsculas e acentos) e o convite no fim (ET-B11). Com a busca, "Todas" sai.
pub fn binder_list(presets: &[Preset], search: &str) -> Vec<String> {
    let wanted = fold(search.trim());
    let mut list = Vec::new();
    if wanted.is_empty() {
        list.push(ALL_BINDERS.to_owned());
    }
    list.extend(
        binders(presets)
            .into_iter()
            .filter(|b| fold(b).contains(&wanted)),
    );
    list.push(ADD_BINDER.to_owned());
    list
}

/// O filtro que vale: uma encadernadora que não existe mais volta para "Todas" (seção 6).
pub fn binder_or_all(presets: &[Preset], binder: Option<&str>) -> Option<String> {
    clean_binder(binder).filter(|l| binders(presets).contains(l))
}

/// Os tamanhos de cada formato com o filtro; um formato sem tamanho fica apagado (regra 4).
pub fn size_choices(presets: &[Preset], binder: Option<&str>) -> Vec<(Format, Vec<Preset>)> {
    let binder = binder_or_all(presets, binder);
    let kept: Vec<Preset> = presets
        .iter()
        .filter(|p| binder.is_none() || clean_binder(p.binder.as_deref()) == binder)
        .cloned()
        .collect();
    formats(&kept)
}

/// Os nomes no seletor de tamanho, como são ("30x30cm"); com o filtro em "Todas", um nome
/// repetido leva a encadernadora entre parênteses (seção 6).
pub fn size_labels(sizes: &[Preset], all_binders: bool) -> Vec<String> {
    sizes
        .iter()
        .map(|p| {
            let repeated = sizes.iter().filter(|q| q.name == p.name).count() > 1;
            match clean_binder(p.binder.as_deref()) {
                Some(binder) if all_binders && repeated => format!("{} ({binder})", p.name),
                _ => p.name.clone(),
            }
        })
        .collect()
}

/// O tamanho que já vem marcado: o do último álbum criado, se ainda existe (regra 1). Um nome de
/// antes da correção de 2026-10-29 ("Horizontal 30 × 20") volta pelo nome novo (ET-B12).
pub fn last_preset(presets: &[Preset], last: Option<&(String, Option<String>)>) -> Option<Preset> {
    let (name, binder) = last?;
    let (name, binder) = crate::preset_names::renamed(name, binder.as_deref())
        .unwrap_or_else(|| (name.clone(), binder.clone()));
    presets
        .iter()
        .find(|p| p.name == name && clean_binder(p.binder.as_deref()) == binder)
        .cloned()
}

/// O preset com exatamente estas medidas, para marcar o tamanho de um álbum aberto (regra 10).
pub fn matching_preset(presets: &[Preset], fields: &SpecFields) -> Option<Preset> {
    presets.iter().find(|p| p.fields == *fields).cloned()
}

/// Os campos que ficam em "Ajustes da gráfica", recolhido (regra 6).
pub fn is_advanced(field: SizeField) -> bool {
    !matches!(field, SizeField::Width | SizeField::Height)
}

/// O bloco recolhido abre sozinho quando o erro é de um campo dele (regra 6).
pub fn advanced_error(form: &SizeForm, limits_error: bool) -> bool {
    limits_error
        || (form.error().is_some() && form.fields.dpi == 0)
        || form.error() == Some(super::size::SAFE_TOO_BIG)
}

/// "30 × 25 cm · 10 a 120 páginas · 300 DPI · sangria 3 mm" (regra 3).
pub fn short_summary(form: &SizeForm, limits: PageLimits) -> Option<String> {
    let spec = form.spec().ok()?;
    let (u, unit) = (form.unit, form.unit.label());
    let mut parts = vec![format!(
        "{} × {} {unit}",
        u.show(spec.page_width().get()),
        u.show(spec.page_height().get())
    )];
    parts.extend(pages(limits));
    parts.push(format!("{} DPI", spec.dpi().get()));
    parts.push(bleed(&form.fields));
    Some(parts.join(" · "))
}

fn pages(limits: PageLimits) -> Option<String> {
    match (limits.min, limits.max) {
        (Some(min), Some(max)) => Some(format!("{min} a {max} páginas")),
        (Some(min), None) => Some(format!("mínimo de {min} páginas")),
        (None, Some(max)) => Some(format!("até {max} páginas")),
        (None, None) => None,
    }
}

/// "sangria 3 mm", "sem sangria" ou só as bordas que têm: "sangria 1,5 mm em cima e embaixo".
fn bleed(fields: &SpecFields) -> String {
    let b = fields.bleed();
    let (outer, top, inner) = (b.outer.get(), b.top_bottom.get(), b.inner.get());
    if outer == top && top == inner {
        return if outer == 0.0 {
            "sem sangria".to_owned()
        } else {
            format!("sangria {} mm", number(outer))
        };
    }
    let sides = [
        (outer, "na externa"),
        (top, "em cima e embaixo"),
        (inner, "na lombada"),
    ];
    let named: Vec<String> = sides
        .iter()
        .filter(|(mm, _)| *mm > 0.0)
        .map(|(mm, side)| format!("{} mm {side}", number(*mm)))
        .collect();
    format!("sangria {}", named.join(", "))
}
