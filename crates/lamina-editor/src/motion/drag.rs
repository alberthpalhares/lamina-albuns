//! Arrastar (spec MV, regra 12): o fantasma mostra a parte da foto que está no quadro, com o lado
//! maior de 120 px e o selo da quantidade numa seleção; o quadro de destino é o mesmo da soltura
//! (o destaque e o destino nunca divergem) e é onde o fantasma pousa; ao reordenar lâminas, as
//! vizinhas abrem espaço.

use lamina_core::{PhotoId, Project};

use crate::view::{Area, FrameKind, PxRect, frame_at, spread_view};

/// Lado maior do fantasma, em pixels.
pub const GHOST_PX: f64 = 120.0;

/// O fantasma de um arrasto.
#[derive(Debug, Clone, PartialEq)]
pub struct Ghost {
    pub photo: PhotoId,
    /// A parte da foto mostrada, em frações da foto inteira.
    pub source: PxRect,
    /// Tamanho na tela, em pixels (o lado maior vale [`GHOST_PX`]).
    pub width: f64,
    pub height: f64,
    /// Quantas fotos vão junto (só numa seleção de duas ou mais).
    pub badge: Option<usize>,
}

fn sized(photo: PhotoId, source: PxRect, aspect: f64, badge: Option<usize>) -> Ghost {
    let (width, height) = if aspect >= 1.0 {
        (GHOST_PX, GHOST_PX / aspect)
    } else {
        (GHOST_PX * aspect, GHOST_PX)
    };
    Ghost {
        photo,
        source,
        width,
        height,
        badge,
    }
}

fn whole() -> PxRect {
    PxRect {
        x: 0.0,
        y: 0.0,
        width: 1.0,
        height: 1.0,
    }
}

/// Área da lâmina em frações da largura, como a janela desenha.
fn unit_area(project: &Project) -> Area {
    let canvas = project.spec.canvas_rect();
    Area {
        width: 1.0,
        height: canvas.height.get() / canvas.width.get(),
        margin: 0.0,
    }
}

/// O fantasma da foto do quadro `frame` da lâmina `index`: a parte que está no quadro, na
/// proporção do quadro.
pub fn frame_ghost(project: &Project, index: usize, frame: usize) -> Option<Ghost> {
    let view = spread_view(project, index, unit_area(project), &[])?;
    let f = view.frames.get(frame)?;
    let FrameKind::Photo { id, source, .. } = &f.kind else {
        return None;
    };
    (f.rect.height > 0.0).then(|| sized(*id, *source, f.rect.width / f.rect.height, None))
}

/// O fantasma da foto `photo` do navegador: a foto inteira, com o selo quando ela faz parte de uma
/// seleção de duas ou mais.
pub fn photo_ghost(project: &Project, photo: PhotoId, selection: &[PhotoId]) -> Option<Ghost> {
    let p = project.photo(photo).ok()?;
    let aspect = f64::from(p.width_px) / f64::from(p.height_px.max(1));
    let badge = (selection.len() >= 2 && selection.contains(&photo)).then_some(selection.len());
    Some(sized(photo, whole(), aspect, badge))
}

/// O quadro da lâmina `index` onde a soltura no ponto (`x`, `y`, em frações da largura) cairia, e
/// o retângulo dele: é o destaque durante o arrasto e o lugar onde o fantasma pousa.
pub fn drop_target(project: &Project, index: usize, x: f64, y: f64) -> Option<(usize, PxRect)> {
    let frame = frame_at(project, index, x, y)?;
    let view = spread_view(project, index, unit_area(project), &[])?;
    Some((frame, view.frames.get(frame)?.rect))
}

/// Reordenar lâminas: aonde vai a lâmina arrastada e quanto cada uma recua durante o arrasto.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Reorder {
    /// A posição final da lâmina arrastada.
    pub to: usize,
    /// Para cada lâmina, quantos lugares ela anda (-1 recua, 1 avança, 0 fica).
    pub shift: Vec<i32>,
}

/// A lâmina `from` arrastada até o vão `gap` (0 = antes da primeira; `count` = depois da última)
/// numa faixa de `count` lâminas.
pub fn reorder(from: usize, gap: usize, count: usize) -> Reorder {
    let gap = gap.min(count);
    let to = if gap > from { gap - 1 } else { gap };
    let shift = (0..count)
        .map(|i| {
            if i > from && i <= to {
                -1
            } else if i >= to && i < from {
                1
            } else {
                0
            }
        })
        .collect();
    Reorder { to, shift }
}

/// Onde uma foto solta numa lâmina cai (spec ML, regra 7).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CrossHit {
    /// Um quadro de foto (vazio ou com foto) e o retângulo dele: o destaque, a soltura e o pouso.
    Frame { frame: usize, rect: PxRect },
    /// A lâmina, fora dos quadros de foto: a foto entra como uma a mais.
    Spread,
}

/// O alvo no ponto (`x`, `y`, em frações da largura) da lâmina `index`. Só quadros de foto contam:
/// um texto conta como fora dos quadros, e um texto por cima de uma foto deixa valer a foto
/// (regra 7). `None` fora da lâmina.
pub fn cross_target(project: &Project, index: usize, x: f64, y: f64) -> Option<CrossHit> {
    let area = unit_area(project);
    if !(0.0..=1.0).contains(&x) || !(0.0..=area.height).contains(&y) {
        return None;
    }
    let view = spread_view(project, index, area, &[])?;
    let hit = view.frames.iter().enumerate().rev().find(|(_, f)| {
        let r = f.rect;
        !matches!(f.kind, FrameKind::Text { .. })
            && x >= r.x
            && x <= r.x + r.width
            && y >= r.y
            && y <= r.y + r.height
    });
    Some(match hit {
        Some((frame, f)) => CrossHit::Frame {
            frame,
            rect: f.rect,
        },
        None => CrossHit::Spread,
    })
}
