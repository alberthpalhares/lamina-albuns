//! Os ímãs a mais do modo organizar (spec EG, regras 7 e 8): mesmo tamanho ao redimensionar e
//! espaçamento igual ao mover, junto com as guias da TX. Medidas em mm, nas coordenadas do
//! `frame_rect`, como em [`crate::snap`].

use lamina_core::{Mm, Project, Rect};

use crate::snap::{Grip, Guides, SnapLine, guides_without, snap};

/// Uma marca do ímã, que a tela desenha só entre os objetos envolvidos (spec EG, saídas).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Mark {
    /// "= 120 mm" ao lado do quadro `rect`: a altura (`tall`) ou a largura que ficou igual.
    Size { rect: Rect, tall: bool, mm: f64 },
    /// Um vão de `from` a `to` no eixo x (`across`) ou no y, na altura `at` do outro eixo.
    Gap {
        across: bool,
        from: f64,
        to: f64,
        at: f64,
    },
}

/// O que atrai o gesto: as guias da TX, os outros quadros e o espaço do álbum.
#[derive(Debug, Clone, Default)]
pub struct Near {
    pub guides: Guides,
    pub others: Vec<Rect>,
    pub gap: f64,
}

impl Near {
    /// O que atrai na lâmina `index`, sem os quadros `skip` (os do gesto).
    pub fn of(project: &Project, index: usize, skip: &[usize]) -> Near {
        let Some(spread) = project.spreads.get(index) else {
            return Near::default();
        };
        let others = spread
            .frames
            .iter()
            .enumerate()
            .filter(|(i, _)| !skip.contains(i))
            .map(|(_, f)| project.frame_rect(f))
            .collect();
        Near {
            guides: guides_without(project, index, skip),
            others,
            gap: project.spread_gap(spread).get(),
        }
    }
}

/// O começo e o fim do retângulo num eixo: x (`across`) ou y.
fn span(r: &Rect, across: bool) -> (f64, f64) {
    if across {
        (r.x.get(), r.right().get())
    } else {
        (r.y.get(), r.bottom().get())
    }
}

fn with_span(r: Rect, across: bool, (a, b): (f64, f64)) -> Rect {
    if across {
        Rect::new(Mm(a), r.y, Mm(b - a), r.height)
    } else {
        Rect::new(r.x, Mm(a), r.width, Mm(b - a))
    }
}

/// Todos os ímãs de um gesto. Num eixo em que o tamanho ou o vão pegou, as guias ficam de fora.
pub fn magnet(
    moved: Rect,
    grip: Grip,
    near: &Near,
    tolerance: f64,
) -> (Rect, Vec<SnapLine>, Vec<Mark>) {
    match grip {
        Grip::Move => {
            let (snapped, lines) = snap(moved, grip, &near.guides, tolerance);
            let mut rect = snapped;
            let mut marks = Vec::new();
            let mut caught = [false; 2];
            for (k, across) in [true, false].into_iter().enumerate() {
                if let Some((d, m)) = gap_axis(moved, near, across, tolerance) {
                    let (a, b) = span(&moved, across);
                    rect = with_span(rect, across, (a + d, b + d));
                    marks.extend(m);
                    caught[k] = true;
                }
            }
            let lines = lines
                .into_iter()
                .filter(|l| !caught[usize::from(!l.vertical)])
                .collect();
            (rect, lines, marks)
        }
        Grip::Resize {
            left,
            right,
            top,
            bottom,
        } => {
            let (sized, caught, marks) = same_size(moved, grip, &near.others, tolerance);
            let rest = Grip::Resize {
                left: left && !caught[0],
                right: right && !caught[0],
                top: top && !caught[1],
                bottom: bottom && !caught[1],
            };
            let (rect, lines) = snap(sized, rest, &near.guides, tolerance);
            (rect, lines, marks)
        }
    }
}

/// Ao redimensionar, a borda segurada encaixa quando a largura ou a altura fica a menos de
/// `tolerance` da de outro quadro (regra 7). Devolve o retângulo, os eixos que pegaram (x, y) e
/// as marcas nos dois quadros.
pub fn same_size(
    rect: Rect,
    grip: Grip,
    others: &[Rect],
    tolerance: f64,
) -> (Rect, [bool; 2], Vec<Mark>) {
    let Grip::Resize {
        left,
        right,
        top,
        bottom,
    } = grip
    else {
        return (rect, [false; 2], Vec::new());
    };
    let mut out = rect;
    let mut caught = [false; 2];
    let mut pairs = Vec::new();
    for (k, (across, held)) in [(true, (left, right)), (false, (top, bottom))]
        .into_iter()
        .enumerate()
    {
        if let Some((s, other)) = size_axis(&rect, held, others, across, tolerance) {
            out = with_span(out, across, s);
            caught[k] = true;
            pairs.push((across, other, s.1 - s.0));
        }
    }
    let marks = pairs
        .into_iter()
        .flat_map(|(across, other, mm)| {
            [out, other].map(|rect| Mark::Size {
                rect,
                tall: !across,
                mm,
            })
        })
        .collect();
    (out, caught, marks)
}

/// O eixo de `rect` com a borda segurada (`lo` ou `hi`) no tamanho do outro quadro mais
/// parecido: o novo começo e fim, e esse quadro.
fn size_axis(
    rect: &Rect,
    (lo, hi): (bool, bool),
    others: &[Rect],
    across: bool,
    tolerance: f64,
) -> Option<((f64, f64), Rect)> {
    if lo == hi {
        return None;
    }
    let (a, b) = span(rect, across);
    let size = b - a;
    let (other, s) = others
        .iter()
        .map(|o| (*o, span(o, across).1 - span(o, across).0))
        .filter(|(_, s)| (s - size).abs() < tolerance)
        .min_by(|p, q| (p.1 - size).abs().total_cmp(&(q.1 - size).abs()))?;
    Some((if hi { (a, a + s) } else { (b - s, b) }, other))
}

/// O meio do trecho em comum de dois retângulos no outro eixo, onde a marca do vão fica.
fn middle(a: &Rect, b: &Rect, across: bool) -> f64 {
    let (a0, a1) = span(a, !across);
    let (b0, b1) = span(b, !across);
    (a0.max(b0) + a1.min(b1)) / 2.0
}

fn gap_mark(lo: &Rect, hi: &Rect, across: bool) -> Mark {
    Mark::Gap {
        across,
        from: span(lo, across).1,
        to: span(hi, across).0,
        at: middle(lo, hi, across),
    }
}

/// O deslocamento do vão igual num eixo (regra 8) e as marcas dos dois vãos.
fn gap_axis(rect: Rect, near: &Near, across: bool, tolerance: f64) -> Option<(f64, Vec<Mark>)> {
    let (a, b) = span(&rect, across);
    let (c0, c1) = span(&rect, !across);
    let mid = (a + b) / 2.0;
    let mut row: Vec<Rect> = near
        .others
        .iter()
        .filter(|o| {
            let (o0, o1) = span(o, !across);
            o0 < c1 && o1 > c0
        })
        .copied()
        .collect();
    row.sort_by(|p, q| span(p, across).0.total_cmp(&span(q, across).0));
    let before = row
        .iter()
        .filter(|o| span(o, across).1 < mid)
        .max_by(|p, q| span(p, across).1.total_cmp(&span(q, across).1));
    let after = row
        .iter()
        .filter(|o| span(o, across).0 > mid)
        .min_by(|p, q| span(p, across).0.total_cmp(&span(q, across).0));
    let gl = before.map(|o| a - span(o, across).1);
    let gr = after.map(|o| span(o, across).0 - b);
    if let (Some(l), Some(r), Some(p), Some(n)) = (gl, gr, before, after)
        && ((r - l) / 2.0).abs() < tolerance
    {
        let d = (r - l) / 2.0;
        let moved = with_span(rect, across, (a + d, b + d));
        return Some((
            d,
            vec![gap_mark(p, &moved, across), gap_mark(&moved, n, across)],
        ));
    }
    // Os vãos de referência: entre vizinhos da mesma linha, e o espaço do álbum.
    let refs = row
        .windows(2)
        .filter_map(|w| {
            let g = span(&w[1], across).0 - span(&w[0], across).1;
            (g > 0.0).then(|| (g, Some(gap_mark(&w[0], &w[1], across))))
        })
        .chain([(near.gap, None)]);
    let mut best: Option<(f64, Vec<Mark>)> = None;
    for (g, reference) in refs {
        let tries = [
            before.zip(gl).map(|(o, l)| (g - l, o, true)),
            after.zip(gr).map(|(o, r)| (r - g, o, false)),
        ];
        for (d, o, left) in tries.into_iter().flatten() {
            if d.abs() < tolerance && best.as_ref().is_none_or(|(b, _)| d.abs() < b.abs()) {
                let moved = with_span(rect, across, (a + d, b + d));
                let own = if left {
                    gap_mark(o, &moved, across)
                } else {
                    gap_mark(&moved, o, across)
                };
                best = Some((d, [Some(own), reference].into_iter().flatten().collect()));
            }
        }
    }
    best
}
