//! Destaque com apoio (spec VL, regra 2): a primeira foto num bloco de cerca de 2/3 e as
//! outras justificadas na faixa que sobra.

use lamina_core::{Mm, Rect};

use super::area::{Block, Ctx, Shaped};
use super::justified::oriented_layouts;
use super::{Arrangement, Family, MAX_PER_PAGE, Scope, SupportSide};

/// Arranjos guardados para a faixa de apoio.
const SUPPORT_KEEP: usize = 4;
/// Fração do bloco ocupada pela foto em destaque.
const HERO_SHARE: f64 = 2.0 / 3.0;

/// Destaque com apoio: a primeira foto num bloco de 2/3 e as outras na faixa ao lado ou embaixo.
pub fn hero(ctx: &Ctx, aspects: &[f64]) -> Vec<Shaped> {
    if aspects.len() < 2 {
        return Vec::new();
    }
    let mut scopes: Vec<(Scope, Rect)> = Vec::new();
    if aspects.len() <= MAX_PER_PAGE {
        scopes.extend(
            ctx.sides()
                .into_iter()
                .map(|s| (Scope::Page(s), ctx.page_area(s))),
        );
    }
    if ctx.spread_mode() {
        scopes.push((Scope::Spread, ctx.spread_area()));
    }
    let mut out = Vec::new();
    for (scope, area) in scopes {
        for support in [SupportSide::Right, SupportSide::Left, SupportSide::Below] {
            out.extend(hero_in(ctx.gap, area, aspects, scope, support));
        }
    }
    out
}

fn hero_in(
    gap: f64,
    area: Rect,
    aspects: &[f64],
    scope: Scope,
    support: SupportSide,
) -> Vec<Shaped> {
    let Some((&a, others)) = aspects.split_first() else {
        return Vec::new();
    };
    let (main_box, strip) = divide(area, support, gap);
    let w = main_box.width.get().min(main_box.height.get() * a);
    let h = w / a;
    let (x, y) = match support {
        SupportSide::Right => (
            main_box.right().get() - w,
            center(main_box.y.get(), main_box.height.get(), h),
        ),
        SupportSide::Left => (
            main_box.x.get(),
            center(main_box.y.get(), main_box.height.get(), h),
        ),
        SupportSide::Below => (
            center(main_box.x.get(), main_box.width.get(), w),
            main_box.bottom().get() - h,
        ),
    };
    let main = Rect::new(Mm(x), Mm(y), Mm(w), Mm(h));
    // A faixa ao lado empilha as fotos em colunas; a de baixo, em linhas.
    let natural = if support == SupportSide::Below {
        Arrangement::Rows
    } else {
        Arrangement::Columns
    };
    oriented_layouts(strip, others, (gap, SUPPORT_KEEP), &[natural])
        .into_iter()
        .map(|(layout, rects)| {
            let family = Family::Hero {
                scope,
                support,
                layout,
            };
            let all = std::iter::once(main).chain(rects).collect();
            (family, vec![Block { area, rects: all }])
        })
        .collect()
}

fn center(start: f64, room: f64, size: f64) -> f64 {
    start + (room - size) / 2.0
}

/// Divide a área no bloco do destaque e na faixa de apoio, com o espaço entre eles.
pub(super) fn divide(area: Rect, support: SupportSide, gap: f64) -> (Rect, Rect) {
    let (x, y, w, h) = (
        area.x.get(),
        area.y.get(),
        area.width.get(),
        area.height.get(),
    );
    match support {
        SupportSide::Below => {
            let main_h = (h - gap) * HERO_SHARE;
            let main = Rect::new(Mm(x), Mm(y), Mm(w), Mm(main_h));
            let strip = Rect::new(Mm(x), Mm(y + main_h + gap), Mm(w), Mm(h - main_h - gap));
            (main, strip)
        }
        SupportSide::Right | SupportSide::Left => {
            let main_w = (w - gap) * HERO_SHARE;
            let strip_w = w - main_w - gap;
            let (main_x, strip_x) = if support == SupportSide::Right {
                (x, x + main_w + gap)
            } else {
                (x + strip_w + gap, x)
            };
            let main = Rect::new(Mm(main_x), Mm(y), Mm(main_w), Mm(h));
            (main, Rect::new(Mm(strip_x), Mm(y), Mm(strip_w), Mm(h)))
        }
    }
}
