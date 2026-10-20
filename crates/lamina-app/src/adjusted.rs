//! A foto ajustada na lâmina grande (spec AF, regra 7): os quadros com giro ou tom ganham a
//! imagem que o editor desenha pela função da exportação, guardada até algo dela mudar. Durante
//! o giro, em um terço da resolução; parado, na resolução cheia.

use std::cell::RefCell;
use std::collections::HashMap;

use lamina_core::{FrameContent, FrameId, PhotoAdjust, PhotoId, Project};
use lamina_editor::adjusted::adjusted_frame;
use slint::{Image, Model, Rgba8Pixel, SharedPixelBuffer};

use crate::SpreadData;
use crate::convert::model;
use crate::state::App;

/// O que, mudando, pede um desenho novo do quadro.
#[derive(Debug, Clone, PartialEq)]
struct Key {
    photo: PhotoId,
    version: i32,
    adjust: PhotoAdjust,
    crop: [u64; 3],
    rect: [u64; 4],
    per_mm: u64,
}

/// A imagem guardada de um quadro e o número dela: a vaga da tela só troca de imagem quando o
/// número muda (spec MV, regra 2), então cada desenho novo ganha um número novo.
struct Drawn {
    key: Key,
    image: Image,
    version: i32,
}

thread_local! {
    static CACHE: RefCell<HashMap<FrameId, Drawn>> = RefCell::new(HashMap::new());
    static NEXT: std::cell::Cell<i32> = const { std::cell::Cell::new(0) };
}

/// O número da imagem ajustada do quadro, se ele tiver uma (negativo: nunca bate com o número de
/// uma foto sem ajuste).
pub fn version(frame: FrameId) -> Option<i32> {
    CACHE.with(|c| c.borrow().get(&frame).map(|d| d.version))
}

fn next_version() -> i32 {
    NEXT.with(|n| {
        let v = n.get().wrapping_add(1);
        n.set(v);
        -2 - (v & 0x3fff_ffff)
    })
}

/// As miniaturas (faixa de lâminas): os quadros ajustados ganham a foto ajustada, desenhada da
/// miniatura, sem guardar (são pequenas e só mudam quando a lâmina muda).
pub fn patch_small(
    project: &Project,
    index: usize,
    images: &HashMap<PhotoId, Image>,
    data: &mut SpreadData,
) {
    let Some(spread) = project.spreads.get(index) else {
        return;
    };
    let per_mm = crate::texts::SMALL / project.spec.canvas_rect().width.get();
    let mut rows: Vec<_> = data.frames.iter().collect();
    let mut changed = false;
    for (k, f) in spread.frames.iter().enumerate() {
        let FrameContent::Photo { photo, .. } = &f.content else {
            continue;
        };
        if project.photo(*photo).is_ok_and(|p| p.adjust.is_neutral()) {
            continue;
        }
        let drawn = images.get(photo).and_then(|img| {
            let buffer = img.to_rgba8()?;
            let pixels = (buffer.width(), buffer.height(), buffer.as_bytes());
            adjusted_frame(project, (index, k), pixels, per_mm)
        });
        if let (Some(a), Some(row)) = (drawn, rows.get_mut(k)) {
            let out = SharedPixelBuffer::<Rgba8Pixel>::clone_from_slice(&a.rgba, a.width, a.height);
            row.img = Image::from_rgba8(out);
            row.has_img = true;
            (row.px, row.py, row.pw, row.ph) = (row.x, row.y, row.w, row.h);
            changed = true;
        }
    }
    if changed {
        data.frames = model(rows);
    }
}

/// Troca a imagem dos quadros ajustados da lâmina `index` pela foto ajustada, e esquece os
/// quadros que não estão mais na tela.
pub fn apply(
    app: &App,
    project: &Project,
    index: usize,
    images: &HashMap<PhotoId, Image>,
    data: &mut SpreadData,
) {
    let Some(spread) = project.spreads.get(index) else {
        return;
    };
    let canvas = project.spec.canvas_rect().width.get();
    let full = crate::texts::BIG / canvas;
    let per_mm = if app.editor.crop_grid().fine {
        full / 3.0
    } else {
        full
    };
    let mut rows: Vec<_> = data.frames.iter().collect();
    let mut changed = false;
    CACHE.with(|cache| {
        let mut cache = cache.borrow_mut();
        cache.retain(|id, _| spread.frames.iter().any(|f| f.id == *id));
        for (k, f) in spread.frames.iter().enumerate() {
            let FrameContent::Photo { photo, crop } = &f.content else {
                continue;
            };
            let Ok(p) = project.photo(*photo) else {
                continue;
            };
            if p.adjust.is_neutral() {
                cache.remove(&f.id);
                continue;
            }
            let Some(screen) = images.get(photo) else {
                continue;
            };
            let r = project.frame_rect(f);
            let key = Key {
                photo: *photo,
                version: app.images.version(*photo),
                adjust: p.adjust,
                crop: [crop.zoom, crop.center_x, crop.center_y].map(f64::to_bits),
                rect: [r.x.get(), r.y.get(), r.width.get(), r.height.get()].map(f64::to_bits),
                per_mm: per_mm.to_bits(),
            };
            let cached = cache
                .get(&f.id)
                .filter(|d| d.key == key)
                .map(|d| d.image.clone());
            let img = cached.or_else(|| {
                let buffer = screen.to_rgba8()?;
                let pixels = (buffer.width(), buffer.height(), buffer.as_bytes());
                let a = adjusted_frame(project, (index, k), pixels, per_mm)?;
                let out =
                    SharedPixelBuffer::<Rgba8Pixel>::clone_from_slice(&a.rgba, a.width, a.height);
                let img = Image::from_rgba8(out);
                let drawn = Drawn {
                    key,
                    image: img.clone(),
                    version: next_version(),
                };
                cache.insert(f.id, drawn);
                Some(img)
            });
            if let (Some(img), Some(row)) = (img, rows.get_mut(k)) {
                // A imagem cobre o quadro estendido inteiro: a foto "inteira" é o próprio quadro.
                row.img = img;
                row.has_img = true;
                (row.px, row.py, row.pw, row.ph) = (row.x, row.y, row.w, row.h);
                changed = true;
            }
        }
    });
    if changed {
        data.frames = model(rows);
    }
}
