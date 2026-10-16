//! Nota de um candidato (spec VL, regra 6): aproveitamento, menor foto, equilíbrio e recorte.

use lamina_core::Rect;

use super::Score;
use super::area::{Ctx, EPS};

/// Calcula a nota de quadros `rects` (em mm) com o recorte `crops` de cada foto.
pub fn score(ctx: &Ctx, rects: &[Rect], crops: &[f64], useful: f64) -> Score {
    let areas: Vec<f64> = rects
        .iter()
        .map(|r| r.width.get() * r.height.get())
        .collect();
    let total_area: f64 = areas.iter().sum();
    let count = areas.len().max(1) as f64;
    let coverage = 50.0 * (total_area / useful).min(1.0);
    let mean = total_area / count;
    let smallest = if mean > 0.0 {
        30.0 * areas.iter().copied().fold(f64::MAX, f64::min) / mean
    } else {
        0.0
    };
    let balance = 20.0 * balance(ctx, rects, &areas);
    let mean_crop = crops.iter().sum::<f64>() / count;
    let crop = -(mean_crop * 100.0).min(15.0);
    Score {
        total: (coverage + smallest + balance + crop).clamp(0.0, 100.0),
        coverage,
        smallest,
        balance,
        crop,
    }
}

/// De 0 a 1: 1 com uma página ativa, com um quadro sobre a lombada, ou com áreas iguais nas
/// duas páginas.
fn balance(ctx: &Ctx, rects: &[Rect], areas: &[f64]) -> f64 {
    if !ctx.both() {
        return 1.0;
    }
    let gutter = ctx.gutter();
    let crosses = |r: &Rect| r.x.get() < gutter - EPS && r.right().get() > gutter + EPS;
    if rects.iter().any(crosses) {
        return 1.0;
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
    1.0 - (left - right).abs() / (left + right)
}
