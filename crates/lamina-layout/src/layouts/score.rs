//! Nota de um candidato (spec SB, regra 6): aproveitamento, menor foto, equilíbrio e recorte.

use lamina_core::{PageSide, Rect};

use super::Score;
use super::area::{Ctx, EPS};
use super::fill::BLEED_MAX_CROP;

/// Página quase cheia diante de uma vazia: fração da página a partir da qual o equilíbrio é cheio.
const FULL_PAGE: f64 = 0.9;
/// Equilíbrio de uma lâmina com várias fotos e alguma sobre a dobra.
const OVER_FOLD: f64 = 0.8;

/// Calcula a nota de quadros `rects` (em mm) com o recorte `crops` de cada foto. `trim` é a área do
/// corte das páginas ativas.
pub fn score(ctx: &Ctx, rects: &[Rect], crops: &[f64], trim: f64) -> Score {
    let areas: Vec<f64> = rects
        .iter()
        .map(|r| r.width.get() * r.height.get())
        .collect();
    let total_area: f64 = areas.iter().sum();
    let count = areas.len().max(1) as f64;
    let coverage = 65.0 * (total_area / trim).min(1.0);
    let mean = total_area / count;
    let smallest = if mean > 0.0 {
        20.0 * areas.iter().copied().fold(f64::MAX, f64::min) / mean
    } else {
        0.0
    };
    let balance = 15.0 * balance(ctx, rects, &areas);
    let mean_crop = crops.iter().sum::<f64>() / count;
    let crop = -(15.0 * mean_crop / BLEED_MAX_CROP).min(15.0);
    Score {
        total: (coverage + smallest + balance + crop).clamp(0.0, 100.0),
        coverage,
        smallest,
        balance,
        crop,
    }
}

/// De 0 a 1: 1 com uma página ativa, com uma foto só sobre a lombada, com áreas iguais nas duas
/// páginas, ou com uma página quase cheia diante de uma vazia.
fn balance(ctx: &Ctx, rects: &[Rect], areas: &[f64]) -> f64 {
    if !ctx.both() {
        return 1.0;
    }
    let gutter = ctx.gutter();
    let crosses = |r: &Rect| r.x.get() < gutter - EPS && r.right().get() > gutter + EPS;
    if rects.iter().any(crosses) {
        return if rects.len() > 1 { OVER_FOLD } else { 1.0 };
    }
    let (mut left, mut right) = (0.0, 0.0);
    for (r, area) in rects.iter().zip(areas) {
        if r.x.get() + r.width.get() / 2.0 < gutter {
            left += area;
        } else {
            right += area;
        }
    }
    if left + right <= 0.0 {
        return 1.0;
    }
    let page = ctx.spec.page_rect(PageSide::Left);
    let page_area = page.width.get() * page.height.get();
    if left.min(right) <= 0.0 && left.max(right) >= FULL_PAGE * page_area {
        return 1.0;
    }
    1.0 - (left - right).abs() / (left + right)
}
