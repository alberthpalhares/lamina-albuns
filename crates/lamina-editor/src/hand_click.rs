//! Um clique sem arrastar com a ferramenta Quadro ou Texto da paleta (correção TX de
//! 2026-10-28): o texto que está embaixo do clique e o quadro no tamanho padrão, centrado nele.

use lamina_core::{FrameContent, Mm, PageSide, Project, Rect};

use crate::hand::{MIN_FRAME_MM, NEW_TEXT_HEIGHT_MM, frame_mm};

/// O gesto foi um clique: menor que o menor quadro nas duas direções.
pub(crate) fn is_click(rect: Rect) -> bool {
    rect.width.get() < MIN_FRAME_MM && rect.height.get() < MIN_FRAME_MM
}

/// O texto de cima sob o ponto (mm) na lâmina `index`.
pub(crate) fn text_at(project: &Project, index: usize, (x, y): (f64, f64)) -> Option<usize> {
    let frames = &project.spreads.get(index)?.frames;
    (0..frames.len()).rev().find(|&i| {
        matches!(frames[i].content, FrameContent::Text(_))
            && frame_mm(project, index, i).is_some_and(|r| {
                (r.x.get()..=r.right().get()).contains(&x)
                    && (r.y.get()..=r.bottom().get()).contains(&y)
            })
    })
}

/// O quadro no tamanho de um novo (texto: 60% da página e duas linhas; foto: 1/3 da página),
/// centrado no ponto (mm). Quem grava o prende ao corte.
pub(crate) fn sized_at(project: &Project, (x, y): (f64, f64), text: bool) -> Rect {
    let left = project.spec.page_rect(PageSide::Left);
    let side = if x < left.right().get() {
        PageSide::Left
    } else {
        PageSide::Right
    };
    let page = project.spec.page_rect(side);
    let (w, h) = if text {
        (page.width.get() * 0.6, NEW_TEXT_HEIGHT_MM)
    } else {
        (page.width.get() / 3.0, page.height.get() / 3.0)
    };
    Rect::new(Mm(x - w / 2.0), Mm(y - h / 2.0), Mm(w), Mm(h))
}
