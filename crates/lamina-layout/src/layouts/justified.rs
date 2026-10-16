//! Layout justificado de um bloco, em linhas ou colunas, sem cortar fotos (TP, regra 3; VL,
//! regra 3), e o mosaico, que enche o bloco com recorte pequeno (VL, regra 2).

use lamina_core::{Mm, Rect};

use super::divisions::{compositions, near_ideal};
use super::{Arrangement, PageLayout};

/// Os arranjos de um bloco que cabem em `area`, linhas primeiro e depois colunas. Se passarem
/// de `keep`, ficam os `keep` com mais área de foto (a ordem entre eles é mantida).
pub fn block_layouts(
    area: Rect,
    aspects: &[f64],
    gap: f64,
    keep: usize,
) -> Vec<(PageLayout, Vec<Rect>)> {
    oriented_layouts(
        area,
        aspects,
        (gap, keep),
        &[Arrangement::Rows, Arrangement::Columns],
    )
}

/// Como [`block_layouts`], só com os tipos de arranjo pedidos. `limits` = (espaço, quantos
/// guardar).
pub fn oriented_layouts(
    area: Rect,
    aspects: &[f64],
    limits: (f64, usize),
    arrangements: &[Arrangement],
) -> Vec<(PageLayout, Vec<Rect>)> {
    let (gap, keep) = limits;
    let mut out = Vec::new();
    if aspects.is_empty() {
        return out;
    }
    for &arrangement in arrangements {
        let near = near_ideal(area, aspects, arrangement);
        for groups in compositions(aspects.len()).iter().filter(|g| near(g.len())) {
            let layout = PageLayout {
                arrangement,
                groups: groups.clone(),
            };
            if let Some(rects) = arrange(area, aspects, &layout, gap) {
                out.push((layout, rects));
            }
        }
    }
    if out.len() > keep {
        let mut indexed: Vec<(f64, usize)> = out
            .iter()
            .enumerate()
            .map(|(i, (_, rects))| (-covered(rects), i))
            .collect();
        indexed.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)));
        let mut chosen = vec![false; out.len()];
        for (_, i) in indexed.iter().take(keep) {
            chosen[*i] = true;
        }
        // Move os escolhidos, na ordem original, sem copiar.
        out = out
            .into_iter()
            .zip(chosen)
            .filter_map(|(layout, keep)| keep.then_some(layout))
            .collect();
    }
    out
}

/// Soma das áreas dos quadros.
pub fn covered(rects: &[Rect]) -> f64 {
    rects.iter().map(|r| r.width.get() * r.height.get()).sum()
}

/// Quadros das fotos arrumadas por `layout` dentro de `area`. `None` se o espaço não couber.
pub fn arrange(area: Rect, aspects: &[f64], layout: &PageLayout, gap: f64) -> Option<Vec<Rect>> {
    oriented(
        area,
        aspects,
        layout,
        |a, x, g| rows(a, x, &layout.groups, g),
        gap,
    )
}

/// Mosaico: as linhas (ou colunas) enchem a área sem sobra, com o mesmo recorte em todas as
/// fotos. `None` se o espaço não couber.
pub fn mosaic(area: Rect, aspects: &[f64], layout: &PageLayout, gap: f64) -> Option<Vec<Rect>> {
    oriented(
        area,
        aspects,
        layout,
        |a, x, g| fill(a, x, &layout.groups, g),
        gap,
    )
}

/// Aplica `build` em linhas; colunas são linhas com os eixos trocados.
fn oriented(
    area: Rect,
    aspects: &[f64],
    layout: &PageLayout,
    build: impl Fn(Rect, &[f64], f64) -> Option<Vec<Rect>>,
    gap: f64,
) -> Option<Vec<Rect>> {
    match layout.arrangement {
        Arrangement::Rows => build(area, aspects, gap),
        Arrangement::Columns => {
            let inverted: Vec<f64> = aspects.iter().map(|a| 1.0 / a).collect();
            let rects = build(transpose(area), &inverted, gap)?;
            Some(rects.into_iter().map(transpose).collect())
        }
    }
}

/// Altura natural de cada linha: a linha ocupa a largura útil com fotos da mesma altura.
fn natural<'a>(
    width: f64,
    aspects: &'a [f64],
    groups: &[usize],
    gap: f64,
) -> Option<Vec<(&'a [f64], f64)>> {
    let mut start = 0;
    let mut out = Vec::with_capacity(groups.len());
    for &count in groups {
        let row = aspects.get(start..start + count)?;
        let room = width - gap * gaps_between(count);
        if room <= 0.0 {
            return None;
        }
        out.push((row, room / row.iter().sum::<f64>()));
        start += count;
    }
    Some(out)
}

/// Linhas justificadas: se o conjunto passar da altura, tudo é reduzido na mesma proporção.
/// O bloco fica centralizado.
fn rows(area: Rect, aspects: &[f64], groups: &[usize], gap: f64) -> Option<Vec<Rect>> {
    let (width, height) = (area.width.get(), area.height.get());
    let natural = natural(width, aspects, groups, gap)?;
    let room_h = height - gap * gaps_between(groups.len());
    let sum_h: f64 = natural.iter().map(|(_, h)| h).sum();
    if room_h <= 0.0 {
        return None;
    }
    let scale = (room_h / sum_h).min(1.0);
    let block_h = scale * sum_h + gap * gaps_between(groups.len());
    let mut y = area.y.get() + (height - block_h) / 2.0;
    let mut rects = Vec::with_capacity(aspects.len());
    for (row, h) in natural {
        let h = h * scale;
        let row_w = row.iter().map(|a| a * h).sum::<f64>() + gap * gaps_between(row.len());
        let mut x = area.x.get() + (width - row_w) / 2.0;
        for a in row {
            rects.push(Rect::new(Mm(x), Mm(y), Mm(a * h), Mm(h)));
            x += a * h + gap;
        }
        y += h + gap;
    }
    Some(rects)
}

/// Linhas que enchem a área: larguras naturais, alturas esticadas (ou encolhidas) por igual.
fn fill(area: Rect, aspects: &[f64], groups: &[usize], gap: f64) -> Option<Vec<Rect>> {
    let natural = natural(area.width.get(), aspects, groups, gap)?;
    let room_h = area.height.get() - gap * gaps_between(groups.len());
    let sum_h: f64 = natural.iter().map(|(_, h)| h).sum();
    if room_h <= 0.0 {
        return None;
    }
    let stretch = room_h / sum_h;
    let mut y = area.y.get();
    let mut rects = Vec::with_capacity(aspects.len());
    for (row, h) in natural {
        let mut x = area.x.get();
        for a in row {
            rects.push(Rect::new(Mm(x), Mm(y), Mm(a * h), Mm(h * stretch)));
            x += a * h + gap;
        }
        y += h * stretch + gap;
    }
    Some(rects)
}

/// Quantos espaços existem entre `count` itens vizinhos.
fn gaps_between(count: usize) -> f64 {
    f64::from(u32::try_from(count.saturating_sub(1)).unwrap_or(u32::MAX))
}

fn transpose(r: Rect) -> Rect {
    Rect::new(r.y, r.x, r.height, r.width)
}

#[cfg(test)]
mod tests;
