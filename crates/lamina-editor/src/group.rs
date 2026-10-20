//! As contas do grupo de quadros do modo organizar (spec EG, regras 4, 5 e 11), em mm: a caixa do
//! grupo, mover sem sair do corte e ampliar a partir do canto oposto, parando no corte e no
//! mínimo de 5 mm de cada quadro.

use lamina_core::{Mm, Rect};

use crate::snap::Grip;

/// O menor e o maior corpo de texto ao ampliar um grupo, em pontos (regra 5).
pub const TEXT_PT: (f64, f64) = (4.0, 300.0);

/// A caixa que envolve os retângulos.
pub fn group_box(rects: &[Rect]) -> Option<Rect> {
    let left = rects.iter().map(|r| r.x.get()).reduce(f64::min)?;
    let top = rects.iter().map(|r| r.y.get()).reduce(f64::min)?;
    let right = rects.iter().map(|r| r.right().get()).reduce(f64::max)?;
    let bottom = rects.iter().map(|r| r.bottom().get()).reduce(f64::max)?;
    Some(Rect::new(
        Mm(left),
        Mm(top),
        Mm(right - left),
        Mm(bottom - top),
    ))
}

/// `v` preso entre `lo` e `hi` (com `lo > hi`, fica `lo`).
fn fit(v: f64, lo: f64, hi: f64) -> f64 {
    if lo > hi { lo } else { v.clamp(lo, hi) }
}

/// O deslocamento (`dx`, `dy`) que mantém a caixa do grupo dentro do corte (regra 4).
pub fn clamp_shift(group: Rect, trim: Rect, dx: f64, dy: f64) -> (f64, f64) {
    (
        fit(
            dx,
            trim.x.get() - group.x.get(),
            trim.right().get() - group.right().get(),
        ),
        fit(
            dy,
            trim.y.get() - group.y.get(),
            trim.bottom().get() - group.bottom().get(),
        ),
    )
}

/// Os retângulos andando (`dx`, `dy`).
pub fn shifted(rects: &[Rect], dx: f64, dy: f64) -> Vec<Rect> {
    rects
        .iter()
        .map(|r| Rect::new(Mm(r.x.get() + dx), Mm(r.y.get() + dy), r.width, r.height))
        .collect()
}

/// O grupo ampliado pela alça `grip` arrastada (`dx`, `dy`) mm: todos pelo mesmo fator, a partir
/// do canto (ou do lado) oposto; nos cantos, proporcional, e com `free` (Shift) cada eixo com o
/// seu fator. Para quando um quadro chega a `min` mm ou a caixa encosta no corte. Devolve os
/// retângulos e os fatores em x e em y.
pub fn scaled(
    rects: &[Rect],
    grip: Grip,
    (dx, dy): (f64, f64),
    free: bool,
    (trim, min): (Rect, f64),
) -> (Vec<Rect>, f64, f64) {
    let (
        Some(b),
        Grip::Resize {
            left,
            right,
            top,
            bottom,
        },
    ) = (group_box(rects), grip)
    else {
        return (rects.to_vec(), 1.0, 1.0);
    };
    let (w, h) = (b.width.get().max(1e-9), b.height.get().max(1e-9));
    let (ax, ay) = (
        if left { b.right().get() } else { b.x.get() },
        if top { b.bottom().get() } else { b.y.get() },
    );
    let horizontal = left || right;
    let vertical = top || bottom;
    let mut sx = if left {
        (w - dx) / w
    } else if right {
        (w + dx) / w
    } else {
        1.0
    };
    let mut sy = if top {
        (h - dy) / h
    } else if bottom {
        (h + dy) / h
    } else {
        1.0
    };
    let corner = horizontal && vertical;
    if corner && !free {
        let s = sx.max(sy);
        (sx, sy) = (s, s);
    }
    // O menor fator: nenhum quadro abaixo do mínimo; o maior: a caixa dentro do corte.
    let small_x = rects
        .iter()
        .map(|r| min / r.width.get())
        .fold(0.0, f64::max);
    let small_y = rects
        .iter()
        .map(|r| min / r.height.get())
        .fold(0.0, f64::max);
    let room_x = (if left {
        ax - trim.x.get()
    } else {
        trim.right().get() - ax
    }) / w;
    let room_y = (if top {
        ay - trim.y.get()
    } else {
        trim.bottom().get() - ay
    }) / h;
    if corner && !free {
        let s = fit(sx, small_x.max(small_y), room_x.min(room_y));
        (sx, sy) = (s, s);
    } else {
        if horizontal {
            sx = fit(sx, small_x, room_x);
        }
        if vertical {
            sy = fit(sy, small_y, room_y);
        }
    }
    let out = rects
        .iter()
        .map(|r| {
            Rect::new(
                Mm(ax + (r.x.get() - ax) * sx),
                Mm(ay + (r.y.get() - ay) * sy),
                Mm(r.width.get() * sx),
                Mm(r.height.get() * sy),
            )
        })
        .collect();
    (out, sx, sy)
}

/// O corpo do texto depois de ampliar o grupo pelos fatores `sx`, `sy`, a partir do corpo do
/// começo do gesto, entre 4 e 300 pt (regra 5).
pub fn scaled_text(start_pt: f64, sx: f64, sy: f64) -> f64 {
    (start_pt * (sx * sy).sqrt()).clamp(TEXT_PT.0, TEXT_PT.1)
}

/// Milímetros por pixel numa lâmina desenhada com `drawn_px` pixels de largura (com a sangria):
/// com zoom 2× a lâmina sai com o dobro de pixels e o mesmo arrasto anda metade dos mm (regra 14).
pub fn mm_per_px(project: &lamina_core::Project, drawn_px: f64) -> Option<f64> {
    let canvas = project.spec.canvas_rect();
    (drawn_px > 0.0).then(|| canvas.width.get() / drawn_px)
}

/// Onde fica o zero das réguas, em mm a partir do canto da lâmina com a sangria: o canto do
/// corte, o mesmo zero dos campos X e Y (seção 4).
pub fn ruler_origin(project: &lamina_core::Project) -> (f64, f64) {
    let (canvas, trim) = (project.spec.canvas_rect(), project.spec.trim_rect());
    (trim.x.get() - canvas.x.get(), trim.y.get() - canvas.y.get())
}
