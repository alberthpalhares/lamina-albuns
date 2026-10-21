//! Marcas que só a tela tem (spec JE, regra 1; a RL acrescenta as dela): o X do quadro vazio nas
//! imagens de tela da lâmina. A exportação nunca chama estas funções, então o arquivo impresso
//! não muda (regra 12).

use image::RgbaImage;
use lamina_core::{FrameContent, Project};
use tiny_skia::{Color, FillRule, Mask, Paint, PathBuilder, PixmapMut, Rect, Stroke, Transform};

use crate::place::{Scale, bleed, pixels};
use crate::style::{box_path, corners};

/// O cinza do quadro vazio, o mesmo da lâmina na janela.
pub const EMPTY_GRAY: [u8; 3] = [0x8a, 0x8a, 0x8a];

/// O X: branco a 35%, como na janela.
const MARK_ALPHA: u8 = 89;

/// Põe o cinza e o X em cada quadro vazio da lâmina `index`, numa imagem dela já pronta (a
/// prévia do renderizador). `stroke_px` é a espessura das diagonais (1,5 na lâmina grande, 1 nas
/// miniaturas).
pub fn draw_empty_marks(image: &mut RgbaImage, project: &Project, index: usize, stroke_px: f32) {
    let Some(spread) = project.spreads.get(index) else {
        return;
    };
    let (width, height) = image.dimensions();
    let canvas = project.spec.canvas_rect();
    let scale = Scale {
        x: f64::from(width) / canvas.width.get(),
        y: f64::from(height) / canvas.height.get(),
    };
    let Some(mut pixmap) = PixmapMut::from_bytes(image.as_mut(), width, height) else {
        return;
    };
    let trim = project.spec.trim_rect();
    for frame in &spread.frames {
        if !matches!(frame.content, FrameContent::Empty) {
            continue;
        }
        let bled = bleed(&project.spec, frame.rect.to_absolute(&trim));
        let Some(at) = pixels(bled.rect, scale, (width, height)) else {
            continue;
        };
        let radii = corners(frame, &bled, scale, at);
        if let Some(outline) = box_path(at, radii) {
            empty_mark(&mut pixmap, &outline, stroke_px);
        }
    }
}

/// Pinta o contorno `outline` de cinza e risca as duas diagonais da caixa dele, recortadas pelo
/// contorno (os cantos arredondados).
pub fn empty_mark(pixmap: &mut PixmapMut, outline: &tiny_skia::Path, stroke_px: f32) {
    let mut gray = Paint::default();
    gray.set_color(Color::from_rgba8(
        EMPTY_GRAY[0],
        EMPTY_GRAY[1],
        EMPTY_GRAY[2],
        255,
    ));
    gray.anti_alias = true;
    pixmap.fill_path(
        outline,
        &gray,
        FillRule::Winding,
        Transform::identity(),
        None,
    );
    let Some(mut clip) = Mask::new(pixmap.width(), pixmap.height()) else {
        return;
    };
    clip.fill_path(outline, FillRule::Winding, true, Transform::identity());
    let b: Rect = outline.bounds();
    let mut x = PathBuilder::new();
    x.move_to(b.left(), b.top());
    x.line_to(b.right(), b.bottom());
    x.move_to(b.right(), b.top());
    x.line_to(b.left(), b.bottom());
    let Some(x) = x.finish() else {
        return;
    };
    let mut white = Paint::default();
    white.set_color(Color::from_rgba8(255, 255, 255, MARK_ALPHA));
    white.anti_alias = true;
    let stroke = Stroke {
        width: stroke_px,
        ..Stroke::default()
    };
    pixmap.stroke_path(&x, &white, &stroke, Transform::identity(), Some(&clip));
}
