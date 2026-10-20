//! As marcas do ímã como a tela desenha (spec EG, saídas): a linha e o texto de cada marca, em
//! frações da largura da lâmina, como as linhas do ímã. O texto vem na unidade da tela.

use lamina_core::{Project, Rect};

use crate::PxRect;
use crate::magnets::Mark;
use crate::new_project::Unit;

/// Folga entre o quadro e a marca de tamanho, em mm.
const AWAY_MM: f64 = 1.5;

/// Uma marca pronta para a tela: a linha (largura ou altura zero) e o texto, com o ponto onde o
/// texto começa.
#[derive(Debug, Clone, PartialEq)]
pub struct MarkView {
    pub line: PxRect,
    pub text: String,
    pub at: (f64, f64),
}

/// As marcas do gesto para a tela, na unidade `unit`.
pub fn mark_views(project: &Project, marks: &[Mark], unit: Unit) -> Vec<MarkView> {
    let canvas = project.spec.canvas_rect();
    let (x0, y0, w) = (canvas.x.get(), canvas.y.get(), canvas.width.get());
    let fx = |v: f64| (v - x0) / w;
    let fy = |v: f64| (v - y0) / w;
    let size = |mm: f64| format!("= {} {}", unit.show(mm), unit.label());
    marks
        .iter()
        .map(|m| match *m {
            Mark::Size { rect, tall, mm } => size_view(rect, tall, (&fx, &fy, w), size(mm)),
            Mark::Gap {
                across,
                from,
                to,
                at,
            } => {
                let text = format!("{} {}", unit.show(to - from), unit.label());
                if across {
                    MarkView {
                        line: px(fx(from), fy(at), (to - from) / w, 0.0),
                        text,
                        at: ((fx(from) + fx(to)) / 2.0, fy(at) + AWAY_MM / w),
                    }
                } else {
                    MarkView {
                        line: px(fx(at), fy(from), 0.0, (to - from) / w),
                        text,
                        at: (fx(at) + AWAY_MM / w, (fy(from) + fy(to)) / 2.0),
                    }
                }
            }
        })
        .collect()
}

type Frac<'a> = (&'a dyn Fn(f64) -> f64, &'a dyn Fn(f64) -> f64, f64);

/// A marca de tamanho ao lado do quadro: à direita para a altura, embaixo para a largura.
fn size_view(rect: Rect, tall: bool, (fx, fy, w): Frac, text: String) -> MarkView {
    let away = AWAY_MM / w;
    if tall {
        let x = fx(rect.right().get()) + away;
        MarkView {
            line: px(x, fy(rect.y.get()), 0.0, rect.height.get() / w),
            text,
            at: (x + away, fy(rect.y.get() + rect.height.get() / 2.0)),
        }
    } else {
        let y = fy(rect.bottom().get()) + away;
        MarkView {
            line: px(fx(rect.x.get()), y, rect.width.get() / w, 0.0),
            text,
            at: (fx(rect.x.get() + rect.width.get() / 2.0), y + away),
        }
    }
}

fn px(x: f64, y: f64, width: f64, height: f64) -> PxRect {
    PxRect {
        x,
        y,
        width,
        height,
    }
}
