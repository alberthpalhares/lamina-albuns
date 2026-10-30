//! Lâmina → imagem (spec `renderizacao`): fundo e fotos recortadas (com os ajustes da spec AF),
//! para impressão ou prévia.

use image::RgbaImage;
use lamina_core::{Frame, FrameContent, Photo, Project};
use rayon::prelude::*;
use tiny_skia::Pixmap;

use crate::frame_photo::{frame_pixels, needed_size, plan};
use crate::place::{Bled, PixelBox, Scale, bleed, pixels};
use crate::style::{Fill, draw_styled, paint_background, styled};
use crate::types::{MIN_PREVIEW, PhotoSource, RenderError, RenderTarget};
use crate::{preview_size, print_size};

/// O que todo quadro precisa para ser desenhado.
struct Scene<'a> {
    project: &'a Project,
    scale: Scale,
    size: (u32, u32),
    photos: &'a dyn PhotoSource,
}

/// Uma foto já reduzida para o seu lugar na tela.
struct Tile<'a> {
    frame: &'a Frame,
    bled: Bled,
    at: PixelBox,
    rgba: Vec<u8>,
}

/// Desenha a lâmina `index` (começando em 0) do projeto, com o fundo de cada página.
pub fn render_spread(
    project: &Project,
    index: usize,
    target: RenderTarget,
    photos: &dyn PhotoSource,
) -> Result<RgbaImage, RenderError> {
    render_spread_with(project, index, target, photos, Fill::Pages)
}

/// Como [`render_spread`], escolhendo como pintar o fundo (spec AP, regra 3).
pub fn render_spread_with(
    project: &Project,
    index: usize,
    target: RenderTarget,
    photos: &dyn PhotoSource,
    fill: Fill,
) -> Result<RgbaImage, RenderError> {
    let spread = project.spreads.get(index).ok_or(RenderError::NoSpread {
        number: index.saturating_add(1),
        count: project.spreads.len(),
    })?;
    let (width, height) = canvas_size(project, target)?;
    let canvas_mm = project.spec.canvas_rect();
    let scene = Scene {
        project,
        scale: Scale {
            x: f64::from(width) / canvas_mm.width.get(),
            y: f64::from(height) / canvas_mm.height.get(),
        },
        size: (width, height),
        photos,
    };
    let mut pixmap = Pixmap::new(width, height).ok_or(RenderError::Canvas { width, height })?;
    paint_background(&mut pixmap, project, spread, fill, scene.scale);
    // Cada foto é lida e reduzida em paralelo; o desenho segue a ordem dos quadros, e o erro
    // devolvido é o do primeiro quadro com problema.
    for group in frame_groups(project, &spread.frames, target) {
        let tiles: Vec<Result<Option<Tile>, RenderError>> =
            group.par_iter().map(|f| tile(&scene, f)).collect();
        for tile in tiles {
            if let Some(tile) = tile? {
                if styled(tile.frame) {
                    draw_styled(
                        &mut pixmap,
                        tile.frame,
                        &tile.bled,
                        scene.scale,
                        tile.at,
                        &tile.rgba,
                    );
                } else {
                    paste(&mut pixmap, &tile);
                }
            }
        }
    }
    crate::text::draw_texts(&mut pixmap, project, spread, scene.scale);
    RgbaImage::from_raw(width, height, pixmap.take()).ok_or(RenderError::Canvas { width, height })
}

/// Quantos bytes de originais uma lâmina de impressão abre ao mesmo tempo (spec LV, regra 13): dá
/// 1 foto de 24 MP ou 2 de 12 MP. A imagem final é a mesma, em qualquer divisão.
const ORIGINALS_AT_ONCE: u64 = 100 * 1024 * 1024;

/// Os quadros em grupos, cada um lido em paralelo e colado antes do próximo: na impressão, a soma
/// dos originais de um grupo cabe no orçamento (ao menos um quadro por grupo). Na prévia, que lê
/// cópias pequenas, um grupo só.
fn frame_groups<'a>(
    project: &Project,
    frames: &'a [Frame],
    target: RenderTarget,
) -> Vec<&'a [Frame]> {
    if !matches!(target, RenderTarget::Print) {
        return vec![frames];
    }
    let bytes = |frame: &Frame| match &frame.content {
        FrameContent::Photo { photo, .. } => project
            .photo(*photo)
            .map_or(0, |p| u64::from(p.width_px) * u64::from(p.height_px) * 4),
        _ => 0,
    };
    let (mut groups, mut start, mut sum) = (Vec::new(), 0, 0);
    for (i, frame) in frames.iter().enumerate() {
        let size = bytes(frame);
        if i > start && sum + size > ORIGINALS_AT_ONCE {
            groups.push(&frames[start..i]);
            (start, sum) = (i, 0);
        }
        sum += size;
    }
    groups.push(&frames[start..]);
    groups
}

fn canvas_size(project: &Project, target: RenderTarget) -> Result<(u32, u32), RenderError> {
    let size = match target {
        RenderTarget::Print => print_size(&project.spec),
        RenderTarget::Preview { max_side } if max_side < MIN_PREVIEW => {
            return Err(RenderError::PreviewTooSmall);
        }
        RenderTarget::Preview { max_side } => preview_size(&project.spec, max_side),
    };
    Ok((size.width, size.height))
}

fn tile<'a>(scene: &Scene, frame: &'a Frame) -> Result<Option<Tile<'a>>, RenderError> {
    let FrameContent::Photo { photo, crop } = &frame.content else {
        return Ok(None);
    };
    let photo = scene.project.photo(*photo)?;
    let spec = &scene.project.spec;
    let rect = frame.rect.to_absolute(&spec.trim_rect());
    let bled = bleed(spec, rect);
    let Some(at) = pixels(bled.rect, scene.scale, scene.size) else {
        return Ok(None);
    };
    let name = file_name(photo);
    // Tamanho que a foto inteira precisa ter, pelas medidas do catálogo, o recorte e o giro.
    let full = (f64::from(photo.width_px), f64::from(photo.height_px));
    let out = ((at.width(), at.height()), scene.scale);
    let p = plan(crop, &photo.adjust, (rect, &bled), full, out);
    let needed = needed_size(p.region, full, p.size);
    let image = scene
        .photos
        .load(photo, needed)
        .map_err(|reason| RenderError::Photo {
            name: name.clone(),
            reason,
        })?;
    if image.width() == 0 || image.height() == 0 {
        return Err(RenderError::Photo {
            name,
            reason: "a imagem lida está vazia".to_owned(),
        });
    }
    let rgba = frame_pixels(&image, crop, &photo.adjust, (rect, &bled), out)
        .map_err(|reason| RenderError::Resize { name, reason })?;
    Ok(Some(Tile {
        frame,
        bled,
        at,
        rgba,
    }))
}

fn file_name(photo: &Photo) -> String {
    photo
        .path
        .file_name()
        .unwrap_or(photo.path.as_os_str())
        .to_string_lossy()
        .into_owned()
}

/// Copia a foto na tela, linha por linha. As fotos são tratadas como opacas (alfa 255), então
/// os pixels entram iguais na tela, que guarda cores pré-multiplicadas.
fn paste(pixmap: &mut Pixmap, tile: &Tile) {
    let stride = pixmap.width() as usize * 4;
    let (x0, w) = (tile.at.x0 as usize, tile.at.width() as usize);
    let data = pixmap.data_mut();
    for (row, src) in tile.rgba.chunks_exact(w * 4).enumerate() {
        let start = (tile.at.y0 as usize + row) * stride + x0 * 4;
        // `pixels` prende o quadro à tela, então a linha sempre cabe.
        debug_assert!(start + w * 4 <= data.len(), "linha fora da tela");
        if let Some(dst) = data.get_mut(start..start + w * 4) {
            for (d, s) in dst
                .as_chunks_mut::<4>()
                .0
                .iter_mut()
                .zip(src.as_chunks::<4>().0)
            {
                d[..3].copy_from_slice(&s[..3]);
                d[3] = 255;
            }
        }
    }
}
