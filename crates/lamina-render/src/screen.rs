//! Marcas que só a tela tem (spec JE, regra 1; spec RL, regra 3): o X do quadro vazio, a marca da
//! foto sumida e o véu das páginas inativas, nas imagens de tela da lâmina (o rolo). A exportação
//! nunca chama estas funções, então o arquivo impresso não muda (regra 12; RL-B24, JE-B1).

use image::RgbaImage;
use lamina_core::{FrameContent, PageSide, PhotoId, Project};
use tiny_skia::{
    Color, FillRule, IntSize, LineCap, Mask, Paint, Path, PathBuilder, Pixmap, PixmapMut, Rect,
    Stroke, Transform,
};

use crate::place::{PixelBox, Scale, bleed, pixels};
use crate::style::{box_path, corners, draw_border};
use crate::types::RenderError;

/// O cinza do quadro vazio, o mesmo da lâmina na janela.
pub const EMPTY_GRAY: [u8; 3] = [0x8a, 0x8a, 0x8a];
/// O véu da página sem conteúdo (lâmina de ponta com página simples), o mesmo da lâmina grande.
pub const INACTIVE_GRAY: [u8; 3] = [0x3a, 0x3a, 0x3a];

/// O X: branco a 35%, como na janela.
const MARK_ALPHA: u8 = 89;
/// O escurecido sobre a foto sumida e a opacidade do desenho branco dela.
const MISSING_VEIL: u8 = 0x66;
const MISSING_ALPHA: u8 = 230;

/// Põe o cinza e o X em cada quadro vazio da lâmina `index`, numa imagem dela já pronta (a
/// prévia do renderizador). `stroke_px` é a espessura das diagonais (1,5 na lâmina grande, 1 nas
/// miniaturas).
pub fn draw_empty_marks(image: &mut RgbaImage, project: &Project, index: usize, stroke_px: f32) {
    let Some(spread) = project.spreads.get(index) else {
        return;
    };
    let (width, height) = image.dimensions();
    let scale = scale_of(project, (width, height));
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
pub fn empty_mark(pixmap: &mut PixmapMut, outline: &Path, stroke_px: f32) {
    let mut gray = Paint::default();
    gray.set_color(rgb(EMPTY_GRAY));
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
    stroke(pixmap, &x, (stroke_px, MARK_ALPHA), Some(&clip));
}

/// O que as marcas da imagem do rolo precisam além do projeto.
#[derive(Debug, Clone, Copy)]
pub struct ScreenMarks<'a> {
    /// As fotos que não estão no caminho salvo.
    pub missing: &'a [PhotoId],
    /// Espessura das linhas, em pixels da imagem (1,5 px da tela na lâmina grande).
    pub line: f32,
}

/// Todas as marcas de tela da lâmina `index` sobre a prévia `image` dela (spec RL, regra 3): o X do
/// quadro vazio (com a borda por cima, como na janela), a marca da foto sumida e, por cima de
/// tudo, o véu das páginas inativas.
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
    // A prévia é opaca: pré-multiplicada ou não, os bytes são os mesmos.
    let mut pixmap = IntSize::from_wh(width, height)
        .and_then(|size| Pixmap::from_vec(image.into_raw(), size))
        .ok_or(RenderError::Canvas { width, height })?;
    let scale = scale_of(project, (width, height));
    for frame in &spread.frames {
        let bled = bleed(&project.spec, project.frame_rect(frame));
        let Some(at) = pixels(bled.rect, scale, (width, height)) else {
            continue;
        };
        let radii = corners(frame, &bled, scale, at);
        let Some(outline) = box_path(at, radii) else {
            continue;
        };
        match &frame.content {
            FrameContent::Empty => {
                empty_mark(&mut pixmap.as_mut(), &outline, marks.line);
                draw_border(&mut pixmap, frame, &bled, scale, radii);
            }
            FrameContent::Photo { photo, .. } if marks.missing.contains(photo) => {
                missing_on(&mut pixmap.as_mut(), (&outline, at), marks.line);
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

fn scale_of(project: &Project, (width, height): (u32, u32)) -> Scale {
    let canvas = project.spec.canvas_rect();
    Scale {
        x: f64::from(width) / canvas.width.get(),
        y: f64::from(height) / canvas.height.get(),
    }
}

fn rgb(c: [u8; 3]) -> Color {
    Color::from_rgba8(c[0], c[1], c[2], 255)
}

/// A foto sumida: escurecida e, no meio, um círculo cortado por uma diagonal (desenho nosso).
fn missing_on(pixmap: &mut PixmapMut, (outline, at): (&Path, PixelBox), line: f32) {
    let mut veil = Paint::default();
    veil.set_color_rgba8(0, 0, 0, MISSING_VEIL);
    veil.anti_alias = true;
    pixmap.fill_path(
        outline,
        &veil,
        FillRule::Winding,
        Transform::identity(),
        None,
    );
    let (w, h) = (at.width() as f32, at.height() as f32);
    let r = (w.min(h) * 0.18).max(3.0 * line);
    let (cx, cy) = (at.x0 as f32 + w / 2.0, at.y0 as f32 + h / 2.0);
    let mut sign = PathBuilder::new();
    sign.push_circle(cx, cy, r);
    let d = r * std::f32::consts::FRAC_1_SQRT_2;
    sign.move_to(cx + d, cy - d);
    sign.line_to(cx - d, cy + d);
    if let Some(sign) = sign.finish() {
        stroke(pixmap, &sign, ((r * 0.16).max(line), MISSING_ALPHA), None);
    }
}

fn stroke(pixmap: &mut PixmapMut, path: &Path, (width, alpha): (f32, u8), clip: Option<&Mask>) {
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

fn fill_box(pixmap: &mut Pixmap, at: PixelBox, c: [u8; 3]) {
    let rect = Rect::from_xywh(
        at.x0 as f32,
        at.y0 as f32,
        at.width() as f32,
        at.height() as f32,
    );
    if let Some(rect) = rect {
        let mut paint = Paint::default();
        paint.set_color(rgb(c));
        pixmap.fill_rect(rect, &paint, Transform::identity(), None);
    }
}
