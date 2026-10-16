//! Gerador de layouts (specs `gerador-de-layouts.md` e `variedade-de-layouts.md`): opções de
//! arranjo para as fotos de uma lâmina, calculadas a partir da proporção de cada foto.

mod area;
mod divisions;
mod families;
mod fill;
mod hero;
mod justified;
mod orders;
mod rank;
mod rotation;
mod score;
mod types;

use std::collections::HashSet;

use lamina_core::{ActivePages, AlbumSpec, Mm};
use rayon::prelude::*;

use area::{Ctx, MARGIN_LEVELS, Raw, aligned};
use rank::{Limits, Sequence, Sort};
pub use types::*;

/// Espaço padrão entre fotos: 8 pt (estudo E2), em mm.
pub const DEFAULT_GAP: Mm = Mm(8.0 * 25.4 / 72.0);

/// Fotos por página (spec VL, regra 1).
pub const MAX_PER_PAGE: usize = 12;

/// Candidatos das famílias do TP (atravessada e por página), do maior para o menor
/// aproveitamento.
pub fn layout_candidates(
    spec: &AlbumSpec,
    active: ActivePages,
    aspects: &[f64],
    gap: Mm,
) -> Result<Vec<LayoutCandidate>, LayoutError> {
    let ctx = context(spec, active, aspects, gap)?;
    let splits = families::splits(&ctx, aspects.len(), false);
    let mut raws = fill::across(&ctx, aspects, None);
    for shaped in families::per_page(&ctx, aspects, &splits, families::BLOCK_KEEP) {
        raws.extend(aligned(shaped, 0, ctx.gutter()).into_iter().take(1));
    }
    let order = (0..aspects.len()).collect();
    let limits = Limits {
        max: usize::MAX,
        exclude: &HashSet::new(),
    };
    let all = rank::finish(
        &ctx,
        aspects,
        &[Sequence { order, raws }],
        Sort::Coverage,
        &limits,
    );
    if all.is_empty() {
        return Err(LayoutError::GapTooLarge);
    }
    Ok(all)
}

/// As duas listas de sugestões da spec VL: na ordem e reordenadas, pela nota e com rodízio.
pub fn layout_options(
    spec: &AlbumSpec,
    active: ActivePages,
    aspects: &[f64],
    options: &LayoutOptions,
) -> Result<LayoutLists, LayoutError> {
    let ctx = context(spec, active, aspects, options.gap)?;
    let original: Vec<usize> = (0..aspects.len()).collect();
    // A ordem original e as reordenações são independentes: montadas em paralelo.
    let reorderings = if options.reordered {
        orders::reorderings(aspects)
    } else {
        Vec::new()
    };
    let mut sequences: Vec<Sequence> = std::iter::once(original)
        .chain(reorderings)
        .collect::<Vec<_>>()
        .into_par_iter()
        .map(|order| sequence(&ctx, aspects, order))
        .collect();
    let first = [sequences.remove(0)];
    let max = options.max_candidates.max(1);
    let none = HashSet::new();
    let limits = Limits {
        max,
        exclude: &none,
    };
    let in_order = rank::finish(&ctx, aspects, &first, Sort::Score, &limits);
    if in_order.is_empty() {
        return Err(LayoutError::GapTooLarge);
    }
    // Um reordenado igual a um "na ordem" (mesmo lugar para cada foto) não se repete.
    let shown: HashSet<u64> = in_order.iter().map(|c| rank::key(&c.frames)).collect();
    let limits = Limits {
        max,
        exclude: &shown,
    };
    let reordered = rank::finish(&ctx, aspects, &sequences, Sort::Score, &limits);
    Ok(LayoutLists {
        in_order,
        reordered,
    })
}

/// Todas as famílias para as fotos na sequência `order`.
fn sequence(base: &Ctx, aspects: &[f64], order: Vec<usize>) -> Sequence {
    let seq: Vec<f64> = order.iter().map(|&i| aspects[i]).collect();
    let mut raws: Vec<Raw> = Vec::new();
    for (level, inset) in (0u8..).zip(MARGIN_LEVELS) {
        let ctx = Ctx { inset, ..*base };
        let splits = families::splits(&ctx, seq.len(), true);
        let shaped = families::per_page(&ctx, &seq, &splits, families::SIDE_KEEP)
            .into_iter()
            .chain(families::whole_spread(&ctx, &seq))
            .chain(hero::hero(&ctx, &seq))
            .chain(families::mirrored(&ctx, &seq, families::SIDE_KEEP));
        for s in shaped {
            raws.extend(aligned(s, level, ctx.gutter()));
        }
    }
    raws.extend(fill::across(base, &seq, Some(fill::BLEED_MAX_CROP)));
    raws.extend(fill::bleed_page(base, &seq));
    raws.extend(fill::mosaics(base, &seq));
    Sequence { order, raws }
}

fn context<'a>(
    spec: &'a AlbumSpec,
    active: ActivePages,
    aspects: &[f64],
    gap: Mm,
) -> Result<Ctx<'a>, LayoutError> {
    let active = normalize(active);
    validate(active, aspects)?;
    Ok(Ctx {
        spec,
        active,
        gap: gap.get().max(0.0),
        inset: 0.0,
    })
}

/// Lâmina sem página ativa conta como a página direita (TP, seção 6).
fn normalize(active: ActivePages) -> ActivePages {
    if active.left || active.right {
        active
    } else {
        ActivePages {
            left: false,
            right: true,
        }
    }
}

fn validate(active: ActivePages, aspects: &[f64]) -> Result<(), LayoutError> {
    if aspects.is_empty() {
        return Err(LayoutError::NoPhotos);
    }
    if let Some(i) = aspects.iter().position(|a| !a.is_finite() || *a <= 0.0) {
        return Err(LayoutError::InvalidAspect(i + 1));
    }
    let pages = usize::from(active.left) + usize::from(active.right);
    if aspects.len() > MAX_PER_PAGE * pages {
        return Err(LayoutError::TooManyPhotos {
            photos: aspects.len(),
            pages,
            capacity: MAX_PER_PAGE * pages,
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests;
#[cfg(test)]
#[path = "tests/vl.rs"]
mod tests_vl;
#[cfg(test)]
#[path = "tests/vl_integrity.rs"]
mod tests_vl_integrity;
#[cfg(test)]
#[path = "tests/vl_score.rs"]
mod tests_vl_score;
