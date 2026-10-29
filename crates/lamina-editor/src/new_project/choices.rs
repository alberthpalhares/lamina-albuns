//! A escolha do tamanho na etapa 1 (spec ET): o filtro por laboratório, os tamanhos de cada
//! formato, o tamanho que já vem marcado e o resumo de uma linha.

use lamina_core::PageLimits;

use super::formats::{Format, formats};
use super::size::{SizeField, SizeForm};
use super::units::number;
use crate::Preset;
use crate::album::SpecFields;

/// O primeiro item do filtro (regra 4).
pub const ALL_LABS: &str = "Todos";
/// O convite do fim da lista de laboratórios (regra 5).
pub const ADD_LAB: &str = "Adicionar o meu laboratório…";

/// O laboratório limpo: sem espaços nas pontas; vazio vira nenhum (seção 6).
pub fn clean_lab(lab: Option<&str>) -> Option<String> {
    lab.map(str::trim)
        .filter(|l| !l.is_empty())
        .map(str::to_owned)
}

/// Os laboratórios dos presets, sem repetir, na ordem em que aparecem.
pub fn labs(presets: &[Preset]) -> Vec<String> {
    let mut found: Vec<String> = Vec::new();
    for lab in presets.iter().filter_map(|p| clean_lab(p.lab.as_deref())) {
        if !found.contains(&lab) {
            found.push(lab);
        }
    }
    found
}

/// O filtro que vale: um laboratório que não existe mais volta para "Todos" (seção 6).
pub fn lab_or_all(presets: &[Preset], lab: Option<&str>) -> Option<String> {
    clean_lab(lab).filter(|l| labs(presets).contains(l))
}

/// Os tamanhos de cada formato com o filtro; um formato sem tamanho fica apagado (regra 4).
pub fn size_choices(presets: &[Preset], lab: Option<&str>) -> Vec<(Format, Vec<Preset>)> {
    let lab = lab_or_all(presets, lab);
    let kept: Vec<Preset> = presets
        .iter()
        .filter(|p| lab.is_none() || clean_lab(p.lab.as_deref()) == lab)
        .cloned()
        .collect();
    formats(&kept)
}

/// Os nomes no seletor de tamanho: "Quadrado 30 × 30" vira "30 × 30"; com o filtro em "Todos",
/// um nome repetido leva o laboratório entre parênteses (seção 6).
pub fn size_labels(format: Format, sizes: &[Preset], all_labs: bool) -> Vec<String> {
    sizes
        .iter()
        .map(|p| {
            let name = p
                .name
                .strip_prefix(format.label())
                .map_or(&*p.name, str::trim);
            let repeated = sizes.iter().filter(|q| q.name == p.name).count() > 1;
            match clean_lab(p.lab.as_deref()) {
                Some(lab) if all_labs && repeated => format!("{name} ({lab})"),
                _ => name.to_owned(),
            }
        })
        .collect()
}

/// O tamanho que já vem marcado: o do último álbum criado, se ainda existe (regra 1).
pub fn last_preset(presets: &[Preset], last: Option<&(String, Option<String>)>) -> Option<Preset> {
    let (name, lab) = last?;
    presets
        .iter()
        .find(|p| p.name == *name && clean_lab(p.lab.as_deref()) == *lab)
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
