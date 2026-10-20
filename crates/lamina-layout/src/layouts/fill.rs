//! Famílias que recortam para encher: atravessada (TP), página sangrada e mosaico (spec VL,
//! regras 2 e 5).

use lamina_core::{PageSide, Rect};

use super::area::{Ctx, Raw};
use super::divisions::compositions;
use super::justified::{block_layouts, mosaic};
use super::{Arrangement, Breathing, Family, MAX_PER_PAGE, PageLayout, Scope};

/// Recorte máximo da foto atravessada e da página sangrada.
pub const BLEED_MAX_CROP: f64 = 0.35;
/// Recorte máximo de cada foto no mosaico.
pub const MOSAIC_MAX_CROP: f64 = 0.15;
/// Arranjos guardados para a página ao lado da sangrada e para cada mosaico.
const KEEP: usize = 4;

/// Fração da foto (proporção `photo`) que fica de fora de um quadro (proporção `frame`).
pub fn crop_of(frame: f64, photo: f64) -> f64 {
    let visible = (frame / photo).min(photo / frame);
    if visible > 1.0 - 1e-9 {
        0.0
    } else {
        1.0 - visible
    }
}

fn aspect(r: &Rect) -> f64 {
    r.width.get() / r.height.get()
}

fn raw(family: Family, rects: Vec<Rect>) -> Raw {
    Raw {
        family,
        breathing: Breathing::default(),
        rects,
    }
}

/// A foto única cobrindo a lâmina inteira. `max_crop` = `None` aceita qualquer recorte (TP).
pub fn across(ctx: &Ctx, aspects: &[f64], max_crop: Option<f64>) -> Vec<Raw> {
    let trim = ctx.spec.trim_rect();
    let [only] = aspects else {
        return Vec::new();
    };
    let fits = max_crop.is_none_or(|max| crop_of(aspect(&trim), *only) <= max);
    if ctx.spread_mode() && fits {
        vec![raw(Family::Across, vec![trim])]
    } else {
        Vec::new()
    }
}

/// Uma foto cobre uma página até o corte. Na página esquerda é a primeira foto, na direita a
/// última (a ordem se mantém); as outras ficam justificadas na outra página. A foto sangrada
/// perde no máximo `max_crop` (o recorte leve da montagem pede 15%; spec NP, regra 6).
pub fn bleed_page(ctx: &Ctx, aspects: &[f64], max_crop: f64) -> Vec<Raw> {
    let n = aspects.len();
    if n == 0 || (n > 1 && (!ctx.both() || n - 1 > MAX_PER_PAGE)) {
        return Vec::new();
    }
    let mut out = Vec::new();
    for side in ctx.sides() {
        let page = ctx.spec.page_rect(side);
        let split = match side {
            PageSide::Left => aspects
                .split_first()
                .map(|(a, rest)| (*a, rest, PageSide::Right)),
            PageSide::Right => aspects
                .split_last()
                .map(|(a, rest)| (*a, rest, PageSide::Left)),
        };
        let Some((bled, rest, other)) = split else {
            continue;
        };
        if crop_of(aspect(&page), bled) > max_crop {
            continue;
        }
        let place = |rest_rects: Vec<Rect>| match side {
            PageSide::Left => std::iter::once(page).chain(rest_rects).collect(),
            PageSide::Right => rest_rects
                .into_iter()
                .chain(std::iter::once(page))
                .collect(),
        };
        if rest.is_empty() {
            out.push(raw(
                Family::BleedPage { side, rest: None },
                place(Vec::new()),
            ));
            continue;
        }
        for (layout, rects) in block_layouts(ctx.page_area(other), rest, ctx.gap, KEEP) {
            let family = Family::BleedPage {
                side,
                rest: Some(layout),
            };
            out.push(raw(family, place(rects)));
        }
    }
    out
}

/// Mosaicos que enchem uma página (todas as fotos nela) ou a lâmina inteira.
pub fn mosaics(ctx: &Ctx, aspects: &[f64]) -> Vec<Raw> {
    let mut scopes: Vec<(Scope, Rect)> = Vec::new();
    if aspects.len() <= MAX_PER_PAGE {
        scopes.extend(
            ctx.sides()
                .into_iter()
                .map(|s| (Scope::Page(s), ctx.page_area(s))),
        );
    }
    if ctx.spread_mode() && aspects.len() >= 2 {
        scopes.push((Scope::Spread, ctx.spread_area()));
    }
    let mut out = Vec::new();
    for (scope, area) in scopes {
        let mut found: Vec<(f64, Raw)> = Vec::new();
        for arrangement in [Arrangement::Rows, Arrangement::Columns] {
            for groups in compositions(aspects.len()) {
                let layout = PageLayout {
                    arrangement,
                    groups: groups.clone(),
                };
                let Some(rects) = mosaic(area, aspects, &layout, ctx.gap) else {
                    continue;
                };
                let worst = rects
                    .iter()
                    .zip(aspects)
                    .map(|(r, a)| crop_of(aspect(r), *a))
                    .fold(0.0, f64::max);
                if worst <= MOSAIC_MAX_CROP {
                    found.push((worst, raw(Family::Mosaic { scope, layout }, rects)));
                }
            }
        }
        found.sort_by(|a, b| a.0.total_cmp(&b.0));
        out.extend(found.into_iter().take(KEEP).map(|(_, r)| r));
    }
    out
}
