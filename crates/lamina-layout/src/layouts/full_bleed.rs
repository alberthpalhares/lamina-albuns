//! Lâminas sem borda (spec SB): as fotos chegam ao corte por fora e só o espaço do álbum as
//! separa. Enchem cada página, o corte inteiro ou um destaque com a faixa de apoio.

use lamina_core::{Mm, PageSide, Rect};

use super::area::{Ctx, Raw};
use super::divisions::compositions;
use super::families::splits;
use super::fill::crop_of;
use super::hero::divide;
use super::justified::mosaic;
use super::rank::gutter_ok;
use super::{Arrangement, Breathing, Family, FullBleed, PageLayout, SupportSide};

/// Arranjos guardados por área e combinações guardadas por divisão das fotos (spec SB, regra 4).
const KEEP: usize = 6;

/// Arranjo que enche uma área, com o maior recorte das fotos dele.
struct Filled {
    worst: f64,
    layout: PageLayout,
    rects: Vec<Rect>,
}

/// Todos os formatos, de 2 fotos em diante, com cada foto perdendo no máximo `max_crop`.
pub fn full_bleed(ctx: &Ctx, aspects: &[f64], max_crop: f64) -> Vec<Raw> {
    if aspects.len() < 2 {
        return Vec::new();
    }
    let mut out = pages(ctx, aspects, max_crop);
    if ctx.spread_mode() {
        out.extend(spread(ctx, aspects, max_crop));
        out.extend(hero(ctx, aspects, max_crop));
    }
    out
}

/// A área de uma página sem borda: recua `gap/2` na lombada se a outra página também tem fotos
/// (spec SB, regra 2).
fn page_bleed(ctx: &Ctx, side: PageSide) -> Rect {
    let page = ctx.spec.page_rect(side);
    if !ctx.both() {
        return page;
    }
    let inset = Mm(ctx.gap / 2.0);
    match side {
        PageSide::Left => Rect::new(page.x, page.y, page.width - inset, page.height),
        PageSide::Right => Rect::new(page.x + inset, page.y, page.width - inset, page.height),
    }
}

/// Os arranjos em linhas e colunas que enchem `area` dentro do limite, do menor para o maior
/// recorte.
fn filled(
    area: Rect,
    aspects: &[f64],
    (gap, max_crop): (f64, f64),
    arrangements: &[Arrangement],
) -> Vec<Filled> {
    let mut out = Vec::new();
    for &arrangement in arrangements {
        for groups in compositions(aspects.len()) {
            let layout = PageLayout {
                arrangement,
                groups: groups.clone(),
            };
            let Some(rects) = mosaic(area, aspects, &layout, gap) else {
                continue;
            };
            let worst = rects
                .iter()
                .zip(aspects)
                .map(|(r, a)| crop_of(aspect(r), *a))
                .fold(0.0, f64::max);
            if worst <= max_crop {
                out.push(Filled {
                    worst,
                    layout,
                    rects,
                });
            }
        }
    }
    out.sort_by(|a, b| a.worst.total_cmp(&b.worst));
    out
}

const BOTH_WAYS: [Arrangement; 2] = [Arrangement::Rows, Arrangement::Columns];

fn aspect(r: &Rect) -> f64 {
    r.width.get() / r.height.get()
}

fn raw(format: FullBleed, rects: Vec<Rect>) -> Raw {
    Raw {
        family: Family::FullBleed(format),
        breathing: Breathing::default(),
        rects,
    }
}

/// Cada página ativa cheia até o corte, com fotos nas duas quando as duas estão ativas.
fn pages(ctx: &Ctx, aspects: &[f64], max_crop: f64) -> Vec<Raw> {
    let mut out = Vec::new();
    for left_count in splits(ctx, aspects.len(), false) {
        let (lefts, rights) = aspects.split_at(left_count);
        let left = page_side(ctx, PageSide::Left, lefts, max_crop);
        let right = page_side(ctx, PageSide::Right, rights, max_crop);
        let mut combined: Vec<(f64, Raw)> = Vec::new();
        for l in &left {
            for r in &right {
                combined.push(combine(l, r));
            }
        }
        combined.sort_by(|a, b| a.0.total_cmp(&b.0));
        out.extend(combined.into_iter().take(KEEP).map(|(_, raw)| raw));
    }
    out
}

/// Os arranjos de uma página; uma página sem fotos é um arranjo vazio. Sem arranjo dentro do
/// limite, a lista vem vazia e a divisão não é oferecida.
fn page_side(ctx: &Ctx, side: PageSide, aspects: &[f64], max_crop: f64) -> Vec<Option<Filled>> {
    if aspects.is_empty() {
        return vec![None];
    }
    filled(
        page_bleed(ctx, side),
        aspects,
        (ctx.gap, max_crop),
        &BOTH_WAYS,
    )
    .into_iter()
    .take(KEEP)
    .map(Some)
    .collect()
}

fn combine(left: &Option<Filled>, right: &Option<Filled>) -> (f64, Raw) {
    let worst = |f: &Option<Filled>| f.as_ref().map_or(0.0, |f| f.worst);
    let layout = |f: &Option<Filled>| f.as_ref().map(|f| f.layout.clone());
    let rects = |f: &Option<Filled>| f.as_ref().map_or_else(Vec::new, |f| f.rects.clone());
    let format = FullBleed::Pages {
        left: layout(left),
        right: layout(right),
    };
    let all = rects(left).into_iter().chain(rects(right)).collect();
    (worst(left).max(worst(right)), raw(format, all))
}

/// Linhas ou colunas no corte inteiro, sem deixar filete na lombada.
fn spread(ctx: &Ctx, aspects: &[f64], max_crop: f64) -> Vec<Raw> {
    filled(
        ctx.spec.trim_rect(),
        aspects,
        (ctx.gap, max_crop),
        &BOTH_WAYS,
    )
    .into_iter()
    .filter(|f| gutter_ok(ctx, &f.rects))
    .take(KEEP)
    .map(|f| raw(FullBleed::Spread(f.layout), f.rects))
    .collect()
}

/// A primeira foto no bloco de destaque e as outras enchendo a faixa, tudo até o corte.
fn hero(ctx: &Ctx, aspects: &[f64], max_crop: f64) -> Vec<Raw> {
    let Some((&first, others)) = aspects.split_first() else {
        return Vec::new();
    };
    let area = ctx.spec.trim_rect();
    let mut out = Vec::new();
    for support in [SupportSide::Right, SupportSide::Left, SupportSide::Below] {
        let (main, strip) = divide(area, support, ctx.gap);
        if crop_of(aspect(&main), first) > max_crop {
            continue;
        }
        // A faixa ao lado empilha as fotos em colunas; a de baixo, em linhas (como no destaque).
        let natural = if support == SupportSide::Below {
            Arrangement::Rows
        } else {
            Arrangement::Columns
        };
        let options = filled(strip, others, (ctx.gap, max_crop), &[natural]);
        for f in options.into_iter().take(KEEP) {
            let rects = std::iter::once(main).chain(f.rects).collect();
            let format = FullBleed::Hero {
                support,
                layout: f.layout,
            };
            out.push(raw(format, rects));
        }
    }
    out
}
