//! O ímã (spec TX, regra 7): perto de uma guia, a borda ou o centro do quadro encaixa nela. As
//! medidas são em mm, nas coordenadas da lâmina com sangria (as do `frame_rect`).

use lamina_core::{Mm, PageSide, Project, Rect};

/// Que parte do quadro o gesto segura.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Grip {
    /// A alça de mover (ou o corpo de um texto).
    Move,
    /// Uma das 8 alças de tamanho: quais bordas ela leva.
    Resize {
        left: bool,
        right: bool,
        top: bool,
        bottom: bool,
    },
}

/// Uma guia que pegou, para a janela desenhar a linha durante o gesto.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SnapLine {
    pub vertical: bool,
    /// Posição em mm (x da linha vertical, y da horizontal).
    pub at: f64,
}

/// As guias de uma lâmina, em mm: posições verticais (x) e horizontais (y).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Guides {
    pub xs: Vec<f64>,
    pub ys: Vec<f64>,
}

fn edges_of(r: &Rect, xs: &mut Vec<f64>, ys: &mut Vec<f64>) {
    xs.extend([r.x.get(), r.right().get()]);
    ys.extend([r.y.get(), r.bottom().get()]);
}

/// Corte, sangria, zonas seguras, dobra, centro das páginas e os outros quadros (com e sem o
/// espaço entre fotos) da lâmina `index`, menos o quadro `moving`.
pub fn guides(project: &Project, index: usize, moving: usize) -> Guides {
    let spec = &project.spec;
    let (mut xs, mut ys) = (Vec::new(), Vec::new());
    edges_of(&spec.trim_rect(), &mut xs, &mut ys);
    edges_of(&spec.canvas_rect(), &mut xs, &mut ys);
    for side in [PageSide::Left, PageSide::Right] {
        edges_of(&spec.safe_rect(side), &mut xs, &mut ys);
        let page = spec.page_rect(side);
        xs.push(page.x.get() + page.width.get() / 2.0);
        ys.push(page.y.get() + page.height.get() / 2.0);
    }
    xs.push(spec.page_rect(PageSide::Left).right().get());
    if let Some(spread) = project.spreads.get(index) {
        let gap = project.spread_gap(spread).get();
        for (i, f) in spread
            .frames
            .iter()
            .enumerate()
            .filter(|(i, _)| *i != moving)
        {
            let r = project.frame_rect(f);
            edges_of(&r, &mut xs, &mut ys);
            xs.extend([
                r.x.get() - gap,
                r.right().get() + gap,
                r.x.get() + r.width.get() / 2.0,
            ]);
            ys.extend([
                r.y.get() - gap,
                r.bottom().get() + gap,
                r.y.get() + r.height.get() / 2.0,
            ]);
            let _ = i;
        }
    }
    Guides { xs, ys }
}

/// A guia mais perto de algum dos `points`, a menos de `tolerance`: (deslocamento, guia).
fn nearest(points: &[f64], guides: &[f64], tolerance: f64) -> Option<(f64, f64)> {
    let mut best: Option<(f64, f64)> = None;
    for p in points {
        for g in guides {
            let d = g - p;
            if d.abs() < tolerance && best.is_none_or(|(b, _)| d.abs() < b.abs()) {
                best = Some((d, *g));
            }
        }
    }
    best
}

/// Encaixa `rect` nas guias. Mover leva o quadro inteiro (bordas e centro); redimensionar leva
/// só as bordas seguradas.
pub fn snap(rect: Rect, grip: Grip, guides: &Guides, tolerance: f64) -> (Rect, Vec<SnapLine>) {
    let (mut x0, mut y0) = (rect.x.get(), rect.y.get());
    let (mut x1, mut y1) = (rect.right().get(), rect.bottom().get());
    let mut lines = Vec::new();
    match grip {
        Grip::Move => {
            if let Some((d, g)) = nearest(&[x0, (x0 + x1) / 2.0, x1], &guides.xs, tolerance) {
                (x0, x1) = (x0 + d, x1 + d);
                lines.push(SnapLine {
                    vertical: true,
                    at: g,
                });
            }
            if let Some((d, g)) = nearest(&[y0, (y0 + y1) / 2.0, y1], &guides.ys, tolerance) {
                (y0, y1) = (y0 + d, y1 + d);
                lines.push(SnapLine {
                    vertical: false,
                    at: g,
                });
            }
        }
        Grip::Resize {
            left,
            right,
            top,
            bottom,
        } => {
            let mut edge = |on: bool, v: &mut f64, list: &[f64], vertical: bool| {
                if let Some((d, g)) = nearest(&[*v], list, tolerance).filter(|_| on) {
                    *v += d;
                    lines.push(SnapLine { vertical, at: g });
                }
            };
            edge(left, &mut x0, &guides.xs, true);
            edge(right, &mut x1, &guides.xs, true);
            edge(top, &mut y0, &guides.ys, false);
            edge(bottom, &mut y1, &guides.ys, false);
        }
    }
    (Rect::new(Mm(x0), Mm(y0), Mm(x1 - x0), Mm(y1 - y0)), lines)
}

/// O retângulo depois de arrastar `dx`, `dy` mm desde o começo do gesto (sem ímã). Ao
/// redimensionar, a borda segurada para antes de passar da outra ou de deixar menos de `min`.
pub fn dragged(start: Rect, grip: Grip, dx: f64, dy: f64, min: f64) -> Rect {
    let (x0, y0) = (start.x.get(), start.y.get());
    let (x1, y1) = (start.right().get(), start.bottom().get());
    let (nx0, ny0, nx1, ny1) = match grip {
        Grip::Move => (x0 + dx, y0 + dy, x1 + dx, y1 + dy),
        Grip::Resize {
            left,
            right,
            top,
            bottom,
        } => (
            if left { (x0 + dx).min(x1 - min) } else { x0 },
            if top { (y0 + dy).min(y1 - min) } else { y0 },
            if right { (x1 + dx).max(x0 + min) } else { x1 },
            if bottom { (y1 + dy).max(y0 + min) } else { y1 },
        ),
    };
    Rect::new(Mm(nx0), Mm(ny0), Mm(nx1 - nx0), Mm(ny1 - ny0))
}
