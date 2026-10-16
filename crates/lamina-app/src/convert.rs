//! Tradução dos dados do `lamina-editor` para as estruturas das telas Slint. Nenhuma decisão
//! aqui: as contas são do editor (regra 1).

use std::rc::Rc;

use lamina_core::{PhotoId, Project, Rgb};
use lamina_editor::{Area, FrameKind, PhotoItem, PxRect, spread_view};
use slint::{Color, Image, ModelRc, VecModel};

use crate::{BoxData, FrameData, PhotoData, SpreadData};

/// Proporção altura ÷ largura da lâmina com a sangria.
pub fn aspect(project: &Project) -> f32 {
    let canvas = project.spec.canvas_rect();
    to_f32(canvas.height.get() / canvas.width.get())
}

// As medidas da tela cabem com folga em f32 (frações e pixels).
#[allow(clippy::cast_possible_truncation)]
fn to_f32(v: f64) -> f32 {
    v as f32
}

pub fn boxed(r: PxRect) -> BoxData {
    BoxData {
        x: to_f32(r.x),
        y: to_f32(r.y),
        w: to_f32(r.width),
        h: to_f32(r.height),
    }
}

fn color(c: Rgb) -> Color {
    Color::from_rgb_u8(c.r, c.g, c.b)
}

/// Pixel da imagem carregada que corresponde à fração `f` do lado `side`.
#[allow(clippy::cast_possible_truncation)]
fn clip(f: f64, side: u32) -> i32 {
    (f * f64::from(side)).round() as i32
}

/// A lâmina `index` em frações da largura da lâmina (a tela escala sozinha), com as imagens
/// que `image` der (imagem de tela na lâmina grande, miniatura na faixa).
pub fn spread_data(
    project: &Project,
    index: usize,
    missing: &[PhotoId],
    image: &dyn Fn(PhotoId) -> Option<Image>,
    warning: bool,
) -> SpreadData {
    let a = f64::from(aspect(project));
    let area = Area {
        width: 1.0,
        height: a,
        margin: 0.0,
    };
    let Some(view) = spread_view(project, index, area, missing) else {
        return SpreadData::default();
    };
    let frames: Vec<FrameData> = view
        .frames
        .iter()
        .map(|f| {
            let mut data = FrameData {
                x: to_f32(f.rect.x),
                y: to_f32(f.rect.y),
                w: to_f32(f.rect.width),
                h: to_f32(f.rect.height),
                tl: to_f32(f.radii[0]),
                tr: to_f32(f.radii[1]),
                br: to_f32(f.radii[2]),
                bl: to_f32(f.radii[3]),
                bw: to_f32(f.border),
                bcolor: color(f.border_color),
                bx: to_f32(f.trim.x),
                by: to_f32(f.trim.y),
                bww: to_f32(f.trim.width),
                bh: to_f32(f.trim.height),
                ..FrameData::default()
            };
            match &f.kind {
                FrameKind::Empty => {}
                FrameKind::Text => data.kind = 2,
                FrameKind::Photo {
                    id,
                    source,
                    missing,
                } => {
                    data.kind = 1;
                    data.missing = *missing;
                    data.label = project
                        .photo(*id)
                        .ok()
                        .and_then(|p| p.path.file_name())
                        .map(|n| n.to_string_lossy().into_owned().into())
                        .unwrap_or_default();
                    if let Some(img) = image(*id).filter(|_| !*missing) {
                        let size = img.size();
                        data.cx = clip(source.x, size.width);
                        data.cy = clip(source.y, size.height);
                        data.cw = clip(source.width, size.width).max(1);
                        data.ch = clip(source.height, size.height).max(1);
                        data.img = img;
                        data.has_img = true;
                    }
                }
            }
            data
        })
        .collect();
    let inactive: Vec<BoxData> = view.inactive.iter().copied().map(boxed).collect();
    SpreadData {
        aspect: to_f32(a),
        bg: color(view.background),
        bg_right: color(view.background_right),
        trim: boxed(view.trim),
        safe_l: boxed(view.safe[0]),
        safe_r: boxed(view.safe[1]),
        spine_x: to_f32(view.spine_x),
        inactive: ModelRc::from(Rc::new(VecModel::from(inactive))),
        frames: ModelRc::from(Rc::new(VecModel::from(frames))),
        warning,
    }
}

/// Uma foto do navegador.
pub fn photo_data(item: &PhotoItem, image: Option<Image>, selected: bool) -> PhotoData {
    PhotoData {
        img: image.unwrap_or_default(),
        name: item.name.clone().into(),
        uses: i32::try_from(item.uses).unwrap_or(i32::MAX),
        selected,
    }
}

/// Lista Slint a partir de um vetor.
pub fn model<T: Clone + 'static>(items: Vec<T>) -> ModelRc<T> {
    ModelRc::from(Rc::new(VecModel::from(items)))
}
