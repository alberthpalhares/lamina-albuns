//! Famílias sem recorte: por página (TP), lâmina inteira justificada e espelhado (spec VL,
//! regra 2). O destaque com apoio fica em `hero.rs`.

use lamina_core::{Mm, PageSide, Rect};

use super::area::{Block, Ctx, Shaped};
use super::justified::{arrange, block_layouts};
use super::{Family, MAX_PER_PAGE, PageLayout};

/// Arranjos guardados por bloco: todos até 4 fotos (TP); os melhores acima disso.
pub const BLOCK_KEEP: usize = 16;
/// Arranjos guardados por página na variedade (VL), onde as duas páginas se combinam em todas
/// as divisões e cada combinação ganha três margens e três alinhamentos.
pub const SIDE_KEEP: usize = 6;

/// Quantas fotos vão para a página esquerda em cada divisão possível. `all_on_one` inclui as
/// divisões com uma página vazia (VL); o TP põe pelo menos uma foto em cada página.
pub fn splits(ctx: &Ctx, n: usize, all_on_one: bool) -> Vec<usize> {
    if !ctx.both() {
        return vec![if ctx.active.left { n } else { 0 }];
    }
    if n == 1 {
        return if all_on_one { vec![0, 1] } else { vec![0] };
    }
    let (lo, hi) = if all_on_one {
        (0, n)
    } else {
        (1, n.saturating_sub(1))
    };
    (lo..=hi)
        .filter(|left| *left <= MAX_PER_PAGE && n - left <= MAX_PER_PAGE)
        .collect()
}

/// Fotos divididas entre as páginas, cada página com seus arranjos justificados.
pub fn per_page(ctx: &Ctx, aspects: &[f64], splits: &[usize], keep: usize) -> Vec<Shaped> {
    let mut out = Vec::new();
    for &left_count in splits {
        let (left_aspects, right_aspects) = aspects.split_at(left_count);
        let lefts = side(ctx, PageSide::Left, left_aspects, keep);
        let rights = side(ctx, PageSide::Right, right_aspects, keep);
        for (left, left_block) in &lefts {
            for (right, right_block) in &rights {
                let family = Family::PerPage {
                    left: left.clone(),
                    right: right.clone(),
                };
                out.push((family, vec![left_block.clone(), right_block.clone()]));
            }
        }
    }
    out
}

fn side(
    ctx: &Ctx,
    page: PageSide,
    aspects: &[f64],
    keep: usize,
) -> Vec<(Option<PageLayout>, Block)> {
    let area = ctx.page_area(page);
    if aspects.is_empty() {
        return vec![(
            None,
            Block {
                area,
                rects: Vec::new(),
            },
        )];
    }
    block_layouts(area, aspects, ctx.gap, keep)
        .into_iter()
        .map(|(layout, rects)| (Some(layout), Block { area, rects }))
        .collect()
}

/// Linhas e colunas justificadas pela lâmina inteira (modo lâmina, duas páginas ativas).
pub fn whole_spread(ctx: &Ctx, aspects: &[f64]) -> Vec<Shaped> {
    if !ctx.spread_mode() {
        return Vec::new();
    }
    let area = ctx.spread_area();
    block_layouts(area, aspects, ctx.gap, BLOCK_KEEP)
        .into_iter()
        .map(|(layout, rects)| (Family::WholeSpread(layout), vec![Block { area, rects }]))
        .collect()
}

/// A página direita espelha a esquerda: mesmo número de fotos e mesma sequência de orientações.
pub fn mirrored(ctx: &Ctx, aspects: &[f64], keep: usize) -> Vec<Shaped> {
    let half = aspects.len() / 2;
    if !ctx.both() || !aspects.len().is_multiple_of(2) || half > MAX_PER_PAGE {
        return Vec::new();
    }
    let (left, right) = aspects.split_at(half);
    if left
        .iter()
        .map(|a| orientation(*a))
        .ne(right.iter().map(|a| orientation(*a)))
    {
        return Vec::new();
    }
    let (left_area, right_area) = (
        ctx.page_area(PageSide::Left),
        ctx.page_area(PageSide::Right),
    );
    let mut out = Vec::new();
    for (layout, left_rects) in block_layouts(left_area, left, ctx.gap, keep) {
        let Some(right_rects) = arrange(right_area, right, &layout, ctx.gap) else {
            continue;
        };
        let reflected = right_rects
            .iter()
            .map(|r| reflect(r, &right_area))
            .collect();
        let blocks = vec![
            Block {
                area: left_area,
                rects: left_rects,
            },
            Block {
                area: right_area,
                rects: reflected,
            },
        ];
        out.push((Family::Mirrored(layout), blocks));
    }
    out
}

/// -1 vertical, 0 quadrada, 1 horizontal.
pub fn orientation(aspect: f64) -> i8 {
    if aspect > 1.05 {
        1
    } else if aspect < 0.95 {
        -1
    } else {
        0
    }
}

fn reflect(r: &Rect, area: &Rect) -> Rect {
    let x = area.x.get() + (area.right().get() - r.right().get());
    Rect::new(Mm(x), r.y, r.width, r.height)
}
