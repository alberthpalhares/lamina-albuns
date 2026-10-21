//! Aparência (spec AP): fundo por página, borda para dentro do quadro e cantos arredondados.
//! A foto sem borda e sem cantos continua sendo colada como antes (os arquivos de sempre não
//! mudam); com eles, é desenhada pelo `tiny-skia` dentro do contorno arredondado.

use lamina_core::{Frame, PageSide, Project, Rgb, Spread};
use tiny_skia::{
    Color, FillRule, FilterQuality, Paint, Path, PathBuilder, Pattern, Pixmap, SpreadMode,
    Transform,
};

use crate::place::{Bled, PixelBox, Scale, pixels};

/// Como pintar o fundo antes dos quadros.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Fill {
    /// Cada metade da lâmina com o fundo da sua página, até a lombada.
    #[default]
    Pages,
    /// A tela inteira com o fundo de uma página: a exportação de página solta, para a sangria da
    /// lombada ter a cor da própria página (spec AP, regra 3).
    Side(PageSide),
}

fn color(c: Rgb) -> Color {
    Color::from_rgba8(c.r, c.g, c.b, 255)
}

/// Pinta o fundo da lâmina.
pub fn paint_background(
    pixmap: &mut Pixmap,
    project: &Project,
    spread: &Spread,
    fill: Fill,
    scale: Scale,
) {
    let (left, right) = (
        spread.page_background(PageSide::Left),
        spread.page_background(PageSide::Right),
    );
    let first = match fill {
        Fill::Side(side) => spread.page_background(side),
        Fill::Pages => left,
    };
    pixmap.fill(color(first));
    if fill != Fill::Pages || left == right {
        return;
    }
    let trim = project.spec.trim_rect();
    let spine = trim.x.get() + trim.width.get() / 2.0;
    #[allow(clippy::cast_possible_truncation)]
    let x = (spine * scale.x).round() as f32;
    let (w, h) = (pixmap.width() as f32, pixmap.height() as f32);
    if let Some(rect) = tiny_skia::Rect::from_xywh(x, 0.0, w - x, h) {
        let mut paint = Paint::default();
        paint.set_color(color(right));
        pixmap.fill_rect(rect, &paint, Transform::identity(), None);
    }
}

/// O quadro tem borda ou cantos (e precisa do desenho com contorno)?
pub fn styled(frame: &Frame) -> bool {
    frame.border.width.get() > 0.0 || frame.radius.get() > 0.0
}

/// Raio de cada canto em pixels (cima-esquerda, cima-direita, baixo-direita, baixo-esquerda).
/// Cantos que caem na sangria ficam retos (regra 5); o raio para na metade do lado menor.
pub(crate) fn corners(frame: &Frame, bled: &Bled, scale: Scale, at: PixelBox) -> [f32; 4] {
    #[allow(clippy::cast_possible_truncation)]
    let r = (frame.radius.get() * scale.x) as f32;
    let limit = at.width().min(at.height()) as f32 / 2.0;
    let r = r.min(limit).max(0.0);
    let [left, top, right, bottom] = bled.grow.map(|g| g > 0.0);
    [
        if left || top { 0.0 } else { r },
        if right || top { 0.0 } else { r },
        if right || bottom { 0.0 } else { r },
        if left || bottom { 0.0 } else { r },
    ]
}

/// Contorno do retângulo `(x, y, w, h)` com os cantos arredondados `r`.
fn rounded(x: f32, y: f32, w: f32, h: f32, r: [f32; 4]) -> Option<Path> {
    // Constante do arco com curva cúbica.
    const K: f32 = 0.552_284_8;
    let [tl, tr, br, bl] = r;
    let mut p = PathBuilder::new();
    p.move_to(x + tl, y);
    p.line_to(x + w - tr, y);
    p.cubic_to(
        x + w - tr + tr * K,
        y,
        x + w,
        y + tr - tr * K,
        x + w,
        y + tr,
    );
    p.line_to(x + w, y + h - br);
    p.cubic_to(
        x + w,
        y + h - br + br * K,
        x + w - br + br * K,
        y + h,
        x + w - br,
        y + h,
    );
    p.line_to(x + bl, y + h);
    p.cubic_to(
        x + bl - bl * K,
        y + h,
        x,
        y + h - bl + bl * K,
        x,
        y + h - bl,
    );
    p.line_to(x, y + tl);
    p.cubic_to(x, y + tl - tl * K, x + tl - tl * K, y, x + tl, y);
    p.close();
    p.finish()
}

pub(crate) fn box_path(at: PixelBox, r: [f32; 4]) -> Option<Path> {
    rounded(
        at.x0 as f32,
        at.y0 as f32,
        at.width() as f32,
        at.height() as f32,
        r,
    )
}

/// Desenha a foto já reduzida (`rgba`, do tamanho de `at`) dentro do contorno arredondado e,
/// por cima, a borda para dentro do quadro no corte final.
pub fn draw_styled(
    pixmap: &mut Pixmap,
    frame: &Frame,
    bled: &Bled,
    scale: Scale,
    at: PixelBox,
    rgba: &[u8],
) {
    let radii = corners(frame, bled, scale, at);
    let size = tiny_skia::IntSize::from_wh(at.width(), at.height());
    let tile = size.and_then(|s| Pixmap::from_vec(opaque(rgba), s));
    if let (Some(tile), Some(path)) = (tile, box_path(at, radii)) {
        let pattern = Pattern::new(
            tile.as_ref(),
            SpreadMode::Pad,
            FilterQuality::Nearest,
            1.0,
            Transform::from_translate(at.x0 as f32, at.y0 as f32),
        );
        let paint = Paint {
            shader: pattern,
            anti_alias: true,
            ..Paint::default()
        };
        pixmap.fill_path(
            &path,
            &paint,
            FillRule::Winding,
            Transform::identity(),
            None,
        );
    }
    draw_border(pixmap, frame, bled, scale, radii);
}

/// Alfa 255 em todos os pixels (as fotos são opacas; a tela guarda cores pré-multiplicadas).
fn opaque(rgba: &[u8]) -> Vec<u8> {
    let mut out = rgba.to_vec();
    for px in out.as_chunks_mut::<4>().0 {
        px[3] = 255;
    }
    out
}

/// A borda: um anel entre o contorno do quadro no corte final e o mesmo contorno recuado da
/// espessura (regra 4).
fn draw_border(pixmap: &mut Pixmap, frame: &Frame, bled: &Bled, scale: Scale, radii: [f32; 4]) {
    let width = frame.border.width.get();
    if width <= 0.0 {
        return;
    }
    let size = (pixmap.width(), pixmap.height());
    let trim = lamina_core::Rect::new(
        lamina_core::Mm(bled.rect.x.get() + bled.grow[0]),
        lamina_core::Mm(bled.rect.y.get() + bled.grow[1]),
        lamina_core::Mm(bled.rect.width.get() - bled.grow[0] - bled.grow[2]),
        lamina_core::Mm(bled.rect.height.get() - bled.grow[1] - bled.grow[3]),
    );
    let Some(outer) = pixels(trim, scale, size) else {
        return;
    };
    #[allow(clippy::cast_possible_truncation)]
    let t = (width * scale.x) as f32;
    let (x, y, w, h) = (
        outer.x0 as f32,
        outer.y0 as f32,
        outer.width() as f32,
        outer.height() as f32,
    );
    let inner_r = radii.map(|r| (r - t).max(0.0));
    let mut ring = PathBuilder::new();
    if let Some(o) = rounded(x, y, w, h, radii) {
        ring.push_path(&o);
    }
    if w > 2.0 * t
        && h > 2.0 * t
        && let Some(i) = rounded(x + t, y + t, w - 2.0 * t, h - 2.0 * t, inner_r)
    {
        ring.push_path(&i);
    }
    if let Some(path) = ring.finish() {
        let mut paint = Paint::default();
        paint.set_color(color(frame.border.color));
        paint.anti_alias = true;
        pixmap.fill_path(
            &path,
            &paint,
            FillRule::EvenOdd,
            Transform::identity(),
            None,
        );
    }
}
