//! Marcas de tela (spec RL, regra 3; spec JE, regra 1): o que só a tela desenha por cima da
//! prévia de uma lâmina, na imagem pronta do rolo: o X do quadro vazio, a marca da foto sumida e
//! o véu das páginas inativas. A exportação nunca chama este módulo (RL-B24, JE-B1).

use image::RgbaImage;
use lamina_core::{FrameContent, PageSide, PhotoId, Project, Rgb};
use tiny_skia::{
    FillRule, IntSize, LineCap, Mask, Paint, Path, PathBuilder, Pixmap, Stroke, Transform,
};

use crate::place::{PixelBox, Scale, bleed, pixels};
use crate::style::{box_path, color, corners, draw_border};
use crate::types::RenderError;

/// O cinza do quadro sem foto, o mesmo da lâmina grande.
pub const EMPTY_GRAY: Rgb = Rgb {
    r: 0x8a,
    g: 0x8a,
    b: 0x8a,
};
/// O véu da página sem conteúdo (lâmina de ponta com página simples), o mesmo da lâmina grande.
pub const INACTIVE_GRAY: Rgb = Rgb {
    r: 0x3a,
    g: 0x3a,
    b: 0x3a,
};
/// As diagonais do X: branco a 35% (spec JE, seção 4).
const X_ALPHA: u8 = 89;
/// O escurecido sobre a foto sumida e a opacidade do desenho branco dela.
const MISSING_VEIL: u8 = 0x66;
const MISSING_ALPHA: u8 = 230;

/// O que as marcas precisam além do projeto.
#[derive(Debug, Clone, Copy)]
pub struct ScreenMarks<'a> {
    /// As fotos que não estão no caminho salvo.
    pub missing: &'a [PhotoId],
    /// Espessura das linhas, em pixels da imagem (1,5 px da tela na lâmina grande).
    pub line: f32,
}

/// Desenha as marcas de tela da lâmina `index` sobre a prévia `image` dela (do tamanho que o
/// renderizador devolveu, com a sangria). As páginas inativas ficam por cima de tudo.
pub fn draw_screen_marks(
    image: RgbaImage,
    project: &Project,
    index: usize,
    marks: &ScreenMarks,
) -> Result<RgbaImage, RenderError> {
    let spread = project.spreads.get(index).ok_or(RenderError::NoSpread {
        number: index.saturating_add(1),
        count: project.spreads.len(),
    })?;
    let (width, height) = image.dimensions();
    let canvas = Canvas::from_image(image)?;
    let mut pixmap = canvas.0;
    let mm = project.spec.canvas_rect();
    let scale = Scale {
        x: f64::from(width) / mm.width.get(),
        y: f64::from(height) / mm.height.get(),
    };
    for frame in &spread.frames {
        let bled = bleed(&project.spec, project.frame_rect(frame));
        let Some(at) = pixels(bled.rect, scale, (width, height)) else {
            continue;
        };
        let radii = corners(frame, &bled, scale, at);
        match &frame.content {
            FrameContent::Empty => {
                empty_on(&mut pixmap, at, radii, marks.line);
                draw_border(&mut pixmap, frame, &bled, scale, radii);
            }
            FrameContent::Photo { photo, .. } if marks.missing.contains(photo) => {
                missing_on(&mut pixmap, at, radii, marks.line);
            }
            _ => {}
        }
    }
    let active = project
        .spec
        .edges()
        .active_pages(index, project.spreads.len());
    for (on, side) in [
        (active.left, PageSide::Left),
        (active.right, PageSide::Right),
    ] {
        let page = pixels(project.spec.page_canvas_rect(side), scale, (width, height));
        if let (false, Some(page)) = (on, page) {
            fill_box(&mut pixmap, page, INACTIVE_GRAY);
        }
    }
    RgbaImage::from_raw(width, height, pixmap.take()).ok_or(RenderError::Canvas { width, height })
}

/// O X do quadro vazio sobre a `image` (opaca): o cinza e as duas diagonais de canto a canto,
/// recortados pelos cantos `radii` do quadro `at` (spec JE, seção 4).
pub fn empty_mark(image: RgbaImage, at: PixelBox, radii: [f32; 4], line: f32) -> RgbaImage {
    let (width, height) = image.dimensions();
    match Canvas::from_image(image) {
        Ok(Canvas(mut pixmap)) => {
            empty_on(&mut pixmap, at, radii, line);
            RgbaImage::from_raw(width, height, pixmap.take()).unwrap_or_default()
        }
        Err(_) => RgbaImage::default(),
    }
}

/// A imagem como tela do `tiny-skia`, sem copiar (a prévia é opaca: pré-multiplicada ou não, os
/// bytes são os mesmos).
struct Canvas(Pixmap);

impl Canvas {
    fn from_image(image: RgbaImage) -> Result<Self, RenderError> {
        let (width, height) = image.dimensions();
        IntSize::from_wh(width, height)
            .and_then(|size| Pixmap::from_vec(image.into_raw(), size))
            .map(Canvas)
            .ok_or(RenderError::Canvas { width, height })
    }
}

fn empty_on(pixmap: &mut Pixmap, at: PixelBox, radii: [f32; 4], line: f32) {
    let Some(outline) = box_path(at, radii) else {
        return;
    };
    let mut paint = Paint::default();
    paint.set_color(color(EMPTY_GRAY));
    paint.anti_alias = true;
    pixmap.fill_path(
        &outline,
        &paint,
        FillRule::Winding,
        Transform::identity(),
        None,
    );
    let (x0, y0, x1, y1) = (at.x0 as f32, at.y0 as f32, at.x1 as f32, at.y1 as f32);
    let mut cross = PathBuilder::new();
    cross.move_to(x0, y0);
    cross.line_to(x1, y1);
    cross.move_to(x1, y0);
    cross.line_to(x0, y1);
    let clip = rounded_clip(pixmap, &outline, radii);
    if let Some(cross) = cross.finish() {
        stroke(pixmap, &cross, line, X_ALPHA, clip.as_ref());
    }
}

fn missing_on(pixmap: &mut Pixmap, at: PixelBox, radii: [f32; 4], line: f32) {
    let Some(outline) = box_path(at, radii) else {
        return;
    };
    let mut veil = Paint::default();
    veil.set_color_rgba8(0, 0, 0, MISSING_VEIL);
    veil.anti_alias = true;
    pixmap.fill_path(
        &outline,
        &veil,
        FillRule::Winding,
        Transform::identity(),
        None,
    );
    // Um círculo cortado por uma diagonal, no meio do quadro: desenho nosso.
    let (w, h) = (at.width() as f32, at.height() as f32);
    let r = (w.min(h) * 0.18).max(3.0 * line);
    let (cx, cy) = (at.x0 as f32 + w / 2.0, at.y0 as f32 + h / 2.0);
    let width = (r * 0.16).max(line);
    let mut sign = PathBuilder::new();
    sign.push_circle(cx, cy, r);
    let d = r * std::f32::consts::FRAC_1_SQRT_2;
    sign.move_to(cx + d, cy - d);
    sign.line_to(cx - d, cy + d);
    if let Some(sign) = sign.finish() {
        stroke(pixmap, &sign, width, MISSING_ALPHA, None);
    }
}

/// Com algum canto arredondado, o que passa do contorno fica de fora.
fn rounded_clip(pixmap: &Pixmap, outline: &Path, radii: [f32; 4]) -> Option<Mask> {
    if radii.iter().all(|r| *r <= 0.0) {
        return None;
    }
    let mut mask = Mask::new(pixmap.width(), pixmap.height())?;
    mask.fill_path(outline, FillRule::Winding, true, Transform::identity());
    Some(mask)
}

fn stroke(pixmap: &mut Pixmap, path: &Path, width: f32, alpha: u8, clip: Option<&Mask>) {
    let mut paint = Paint::default();
    paint.set_color_rgba8(255, 255, 255, alpha);
    paint.anti_alias = true;
    let style = Stroke {
        width,
        line_cap: LineCap::Butt,
        ..Stroke::default()
    };
    pixmap.stroke_path(path, &paint, &style, Transform::identity(), clip);
}

fn fill_box(pixmap: &mut Pixmap, at: PixelBox, rgb: Rgb) {
    let rect = tiny_skia::Rect::from_xywh(
        at.x0 as f32,
        at.y0 as f32,
        at.width() as f32,
        at.height() as f32,
    );
    if let Some(rect) = rect {
        let mut paint = Paint::default();
        paint.set_color(color(rgb));
        pixmap.fill_rect(rect, &paint, Transform::identity(), None);
    }
}
