//! A imagem da lâmina do rolo (spec RL, regras 3 a 5): a prévia da impressão com as marcas de
//! tela, sem ler o original; a exportação sem marcas; e a velocidade (RL-B8, RL-B24 e RL-B27).

use std::path::Path;
use std::time::{Duration, Instant};

use image::{Rgba, RgbaImage};
use lamina_core::testing::{HEAVY_TRIES, budget};
use lamina_core::{
    AlbumEdge, Crop, FrameContent, PageSide, PhotoAdjust, PhotoId, Project, RelRect, Rgb, TextBox,
};
use lamina_render::{
    EMPTY_GRAY, Fill, INACTIVE_GRAY, PixelBox, RenderTarget, Scale, bleed, pixels,
    render_spread_with,
};

use super::common::Base;
use crate::motion::Tier;
use crate::{ExportFormat, ImageSize, RollSource, export_project, render_spread_image};

/// Um álbum de uma lâmina que começa com página simples (a esquerda fica inativa), com as fotos
/// já no cache e os originais trocados por lixo: quem ler um original falha.
fn album(base: &Base) -> (Project, PhotoId) {
    let mut project = base.project(2);
    project.spec = project
        .spec
        .clone()
        .with_edges(AlbumEdge::SinglePage, AlbumEdge::FullSpread);
    let id = project.spreads[0].id;
    project.spreads[0].frames.clear();
    let photos: Vec<PhotoId> = project.photos.iter().map(|p| p.id).collect();
    let (turned, gone) = (photos[0], photos[1]);
    if let Some(p) = project.photos.iter_mut().find(|p| p.id == turned) {
        p.adjust = PhotoAdjust {
            angle_tenths: 50,
            bw: true,
            ..PhotoAdjust::default()
        };
    }
    let photo = |photo| FrameContent::Photo {
        photo,
        crop: Crop::default(),
    };
    let mut text = TextBox::new(Rgb { r: 0, g: 0, b: 0 });
    text.text = "Ana e Rui".into();
    let frames = [
        ([0.55, 0.1, 0.2, 0.35], photo(turned)),
        ([0.55, 0.55, 0.2, 0.35], photo(gone)),
        ([0.78, 0.1, 0.18, 0.35], FrameContent::Empty),
        ([0.78, 0.55, 0.18, 0.2], FrameContent::Text(text)),
    ];
    for (r, content) in frames {
        let rect = RelRect::new(r[0], r[1], r[2], r[3]).unwrap();
        project.add_frame(id, rect, content).unwrap();
    }
    for p in &project.photos {
        std::fs::write(&p.path, b"nao e uma foto").unwrap();
    }
    (project, gone)
}

/// Os pixels de `rect` (mm) na imagem, com uma folga de 2 px (o contorno é suavizado).
fn pixel_box(project: &Project, image: &RgbaImage, rect: lamina_core::Rect) -> PixelBox {
    let mm = project.spec.canvas_rect();
    let scale = Scale {
        x: f64::from(image.width()) / mm.width.get(),
        y: f64::from(image.height()) / mm.height.get(),
    };
    let b = pixels(rect, scale, image.dimensions()).unwrap();
    PixelBox {
        x0: b.x0.saturating_sub(2),
        y0: b.y0.saturating_sub(2),
        x1: b.x1 + 2,
        y1: b.y1 + 2,
    }
}

/// O quadro `n` da lâmina 1, estendido pela sangria como a tela o desenha.
fn frame_box(project: &Project, image: &RgbaImage, n: usize) -> PixelBox {
    let frame = &project.spreads[0].frames[n];
    let rect = bleed(&project.spec, project.frame_rect(frame)).rect;
    pixel_box(project, image, rect)
}

fn inside(b: PixelBox, x: u32, y: u32) -> bool {
    x >= b.x0 && x < b.x1 && y >= b.y0 && y < b.y1
}

fn luma(p: &Rgba<u8>) -> u32 {
    u32::from(p[0]) + u32::from(p[1]) + u32::from(p[2])
}

#[test]
fn rl_b8_imagem_e_a_previa_da_impressao_com_as_marcas_sem_ler_o_original() {
    let base = Base::with_photos(4, (600, 400));
    let (project, gone) = album(&base);
    let source = RollSource::new(&project, 0, &base.cache(), Tier::Screen);
    let size = ImageSize {
        width: 1200,
        shown: 700.0,
    };
    let image = render_spread_image(&project, (0, size), &source, &[gone]).unwrap();
    assert_eq!(image.pixels.width(), 1200);
    assert!(image.complete, "as imagens de tela do cache bastaram");
    let target = RenderTarget::Preview {
        max_side: image.pixels.width(),
    };
    let same = RollSource::new(&project, 0, &base.cache(), Tier::Screen);
    let plain = render_spread_with(&project, 0, target, &same, Fill::Pages).unwrap();
    assert_eq!(plain.dimensions(), image.pixels.dimensions());
    let (missing, empty) = (
        frame_box(&project, &plain, 1),
        frame_box(&project, &plain, 2),
    );
    let page = project.spec.page_canvas_rect(PageSide::Left);
    let inactive = pixel_box(&project, &plain, page);
    for (x, y, p) in plain.enumerate_pixels() {
        let marked = inside(missing, x, y) || inside(empty, x, y) || inside(inactive, x, y);
        if !marked {
            assert_eq!(image.pixels.get_pixel(x, y), p, "pixel ({x}, {y})");
        }
    }
    // O X no quadro vazio, a marca na foto sumida e o véu na página inativa.
    let center = |b: PixelBox| ((b.x0 + b.x1) / 2, (b.y0 + b.y1) / 2);
    let (cx, cy) = center(empty);
    let gray = Rgba([EMPTY_GRAY.r, EMPTY_GRAY.g, EMPTY_GRAY.b, 255]);
    assert!(luma(image.pixels.get_pixel(cx, cy)) > luma(&gray));
    let (mx, my) = (missing.x0 + 6, missing.y0 + 6);
    assert!(luma(image.pixels.get_pixel(mx, my)) < luma(plain.get_pixel(mx, my)));
    let veil = Rgba([INACTIVE_GRAY.r, INACTIVE_GRAY.g, INACTIVE_GRAY.b, 255]);
    assert_eq!(*image.pixels.get_pixel(10, plain.height() / 2), veil);
}

#[test]
fn rl_b8_sem_cache_o_quadro_fica_cinza_e_a_imagem_incompleta() {
    let base = Base::with_photos(4, (600, 400));
    let (project, _) = album(&base);
    let empty = crate::Cache::new(base.path("cache-vazio"));
    let source = RollSource::new(&project, 0, &empty, Tier::Small);
    let size = ImageSize {
        width: 384,
        shown: 700.0,
    };
    let image = render_spread_image(&project, (0, size), &source, &[]).unwrap();
    assert!(!image.complete);
    let b = frame_box(&project, &image.pixels, 0);
    let (x, y) = ((b.x0 + b.x1) / 2, (b.y0 + b.y1) / 2);
    let p = image.pixels.get_pixel(x, y);
    // Cinza (a foto em preto e branco e girada continua cinza).
    assert!(
        p[0].abs_diff(p[1]) <= 2 && p[1].abs_diff(p[2]) <= 2,
        "{p:?}"
    );
}

fn export_all(project: &Project, out: &Path) -> Vec<Vec<u8>> {
    let mut files = Vec::new();
    for format in [ExportFormat::Jpeg, ExportFormat::Tiff, ExportFormat::Pdf] {
        let options = lamina_export::ExportOptions {
            format,
            overwrite: true,
            ..lamina_export::ExportOptions::default()
        };
        let report = export_project(project, out, &options, &|_, _| {}).unwrap();
        files.extend(report.files.iter().map(|f| std::fs::read(f).unwrap()));
    }
    files
}

#[test]
fn rl_b24_exportacao_nao_tem_marcas_de_tela() {
    let base = Base::with_photos(4, (600, 400));
    let mut project = base.project(2);
    let id = project.spreads[0].id;
    let rect = RelRect::new(0.7, 0.7, 0.2, 0.2).unwrap();
    project.add_frame(id, rect, FrameContent::Empty).unwrap();
    let before = export_all(&project, &base.path("antes"));
    let source = RollSource::new(&project, 0, &base.cache(), Tier::Screen);
    let size = ImageSize {
        width: 800,
        shown: 800.0,
    };
    render_spread_image(&project, (0, size), &source, &[]).unwrap();
    let after = export_all(&project, &base.path("depois"));
    assert_eq!(before, after, "iguais byte a byte");
    // O quadro vazio sai com a cor do fundo da página.
    let jpeg = image::load_from_memory(&after[0]).unwrap().into_rgb8();
    let (w, h) = jpeg.dimensions();
    let p = jpeg.get_pixel(w * 4 / 5, h * 4 / 5);
    let bg = project.spreads[0].page_background(PageSide::Right);
    assert!(p[0].abs_diff(bg.r) <= 3 && p[1].abs_diff(bg.g) <= 3 && p[2].abs_diff(bg.b) <= 3);
}

#[test]
fn rl_b27_imagem_de_tela_de_2048_px_rapido() {
    let base = Base::with_photos(24, (1600, 1067));
    let mut project = base.project(3);
    // Doze fotos na lâmina 1, todas com imagem de tela no cache.
    let id = project.spreads[0].id;
    project.spreads[0].frames.clear();
    let photos: Vec<PhotoId> = project.photos.iter().take(12).map(|p| p.id).collect();
    for (i, photo) in photos.iter().enumerate() {
        #[allow(clippy::cast_precision_loss)]
        let (col, row) = ((i % 4) as f64, (i / 4) as f64);
        let rect = RelRect::new(0.02 + col * 0.245, 0.03 + row * 0.32, 0.23, 0.3).unwrap();
        let content = FrameContent::Photo {
            photo: *photo,
            crop: Crop::default(),
        };
        project.add_frame(id, rect, content).unwrap();
    }
    let cache = base.cache();
    let size = ImageSize {
        width: 2048,
        shown: 1170.0,
    };
    let painter = crate::RollPainter::new(2, cache.clone());
    let mut best = Duration::MAX;
    for _ in 0..HEAVY_TRIES {
        let source = RollSource::new(&project, 0, &cache, Tier::Screen);
        let started = Instant::now();
        let image = painter
            .run(|| render_spread_image(&project, (0, size), &source, &[]))
            .unwrap();
        best = best.min(started.elapsed());
        assert!(image.complete);
        if best < budget(150) {
            break;
        }
    }
    assert!(best < budget(150), "a imagem de 2.048 px levou {best:?}");
}

#[test]
#[allow(clippy::cast_possible_truncation)]
fn foto_rgb_vira_rgba_opaca_com_as_mesmas_cores() {
    let rgb = image::RgbImage::from_fn(3, 2, |x, y| image::Rgb([x as u8 * 40, y as u8 * 90, 7]));
    let out = crate::roll_image::rgba(image::DynamicImage::ImageRgb8(rgb.clone()));
    assert_eq!(out, image::DynamicImage::ImageRgb8(rgb).into_rgba8());
    let gray =
        image::DynamicImage::ImageLuma8(image::GrayImage::from_pixel(2, 2, image::Luma([9])));
    assert_eq!(
        *crate::roll_image::rgba(gray).get_pixel(1, 1),
        Rgba([9, 9, 9, 255])
    );
}
