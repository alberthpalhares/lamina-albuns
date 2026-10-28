//! Os ajustes da foto no renderizador (spec AF, regras 5 a 7): a fórmula do tom, nenhum canto
//! vazio ao girar, a tela igual à impressão, sem ajuste nada muda, mesma entrada e mesmos
//! pixels, e a velocidade (AF-B3, AF-B6, AF-B7, AF-B8, AF-B14 e AF-B16).

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

use image::{Rgba, RgbaImage};
use lamina_core::testing::{HEAVY_TRIES, TRIES, budget};
use lamina_core::{
    AlbumSpec, Crop, Dpi, FrameContent, Mm, Photo, PhotoAdjust, PhotoId, Project, RelRect, Rgb,
};

use crate::{
    RenderTarget, Scale, apply_tone, bleed, frame_pixels, gray, region, render_spread, tone_table,
};

/// Um álbum com uma lâmina e fotos em memória.
struct Album {
    project: Project,
    images: HashMap<PathBuf, Arc<RgbaImage>>,
}

impl Album {
    fn new(cm: f64, dpi: u32, bleed_mm: f64) -> Self {
        let spec = AlbumSpec::new(Mm::from_cm(cm), Mm::from_cm(cm), Dpi::new(dpi).unwrap())
            .unwrap()
            .with_bleed(Mm(bleed_mm))
            .unwrap();
        let mut project = Project::new("teste", spec);
        project.add_spread();
        project.spreads[0].background = Rgb {
            r: 255,
            g: 0,
            b: 255,
        };
        Album {
            project,
            images: HashMap::new(),
        }
    }

    fn photo(&mut self, name: &str, image: RgbaImage, adjust: PhotoAdjust) -> PhotoId {
        let id = self.project.add_photo(name, image.width(), image.height());
        if let Some(p) = self.project.photos.last_mut() {
            p.adjust = adjust;
        }
        self.images.insert(PathBuf::from(name), Arc::new(image));
        id
    }

    fn frame(&mut self, photo: PhotoId, rect: [f64; 4], crop: Crop) {
        let id = self.project.spreads[0].id;
        let rect = RelRect::new(rect[0], rect[1], rect[2], rect[3]).unwrap();
        let content = FrameContent::Photo { photo, crop };
        self.project.add_frame(id, rect, content).unwrap();
    }

    fn render(&self, target: RenderTarget) -> RgbaImage {
        let source = |p: &Photo, _: (u32, u32)| {
            self.images
                .get(&p.path)
                .cloned()
                .ok_or_else(|| "arquivo sumiu".to_owned())
        };
        render_spread(&self.project, 0, target, &source).unwrap()
    }
}

fn turned(tenths: i32) -> PhotoAdjust {
    PhotoAdjust {
        angle_tenths: tenths,
        ..PhotoAdjust::default()
    }
}

/// Um degradê suave (a comparação entre escalas não sofre com bordas duras).
fn gradient(w: u32, h: u32) -> RgbaImage {
    RgbaImage::from_fn(w, h, |x, y| {
        let r = u8::try_from(x * 255 / w.max(1)).unwrap_or(255);
        let g = u8::try_from(y * 255 / h.max(1)).unwrap_or(255);
        Rgba([r, g, 128, 255])
    })
}

#[test]
fn af_b7_formula_do_tom() {
    let bright = tone_table(&PhotoAdjust {
        brightness: 50,
        ..PhotoAdjust::default()
    });
    assert_eq!(bright[128], 160);
    let hard = tone_table(&PhotoAdjust {
        contrast: 100,
        ..PhotoAdjust::default()
    });
    let soft = tone_table(&PhotoAdjust {
        contrast: -100,
        ..PhotoAdjust::default()
    });
    assert_eq!((hard[64], soft[64]), (40, 88));
    for b in [-100, -50, 0, 50, 100] {
        for c in [-100, -30, 0, 30, 100] {
            let t = tone_table(&PhotoAdjust {
                brightness: b,
                contrast: c,
                ..PhotoAdjust::default()
            });
            assert_eq!((t[0], t[255]), (0, 255), "pontas fixas em {b}/{c}");
            assert!(t.windows(2).all(|w| w[0] <= w[1]), "a curva nunca inverte");
        }
    }
    // Neutro não toca na foto; preto e branco deixa R = G = B.
    let mut px = vec![200, 30, 90, 255, 0, 0, 0, 255, 255, 255, 255, 255];
    let before = px.clone();
    apply_tone(&mut px, &PhotoAdjust::default());
    assert_eq!(px, before);
    apply_tone(
        &mut px,
        &PhotoAdjust {
            bw: true,
            ..PhotoAdjust::default()
        },
    );
    assert!(px.chunks(4).all(|p| p[0] == p[1] && p[1] == p[2]));
    assert_eq!(
        &px[4..12],
        &[0, 0, 0, 255, 255, 255, 255, 255],
        "preto e branco puros"
    );
    assert_eq!(px[0], gray(200, 30, 90));
}

#[test]
fn af_b3_nenhum_pixel_do_quadro_sai_com_o_fundo() {
    // Foto de cor sólida, fundo magenta: qualquer canto vazio apareceria magenta.
    for (rect, degrees) in [
        ([0.1, 0.1, 0.3, 0.6], -450),
        ([0.0, 0.0, 0.5, 1.0], -73),
        ([0.55, 0.2, 0.2, 0.4], 1),
        ([0.6, 0.0, 0.1, 1.0], 120),
        ([0.5, 0.0, 0.5, 1.0], 450),
    ] {
        let mut album = Album::new(10.0, 100, 3.0);
        let id = album.photo(
            "a.jpg",
            RgbaImage::from_pixel(600, 400, Rgba([0, 160, 0, 255])),
            turned(degrees),
        );
        album.frame(
            id,
            rect,
            Crop {
                center_x: 0.0,
                ..Crop::default()
            },
        );
        let img = album.render(RenderTarget::Print);
        let p = &album.project;
        let frame = &p.spreads[0].frames[0];
        let bled = bleed(&p.spec, p.frame_rect(frame));
        let canvas = p.spec.canvas_rect();
        let per = f64::from(img.width()) / canvas.width.get();
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let px = |mm: f64| (mm * per).round() as u32;
        let (x0, y0) = (px(bled.rect.x.get()), px(bled.rect.y.get()));
        let (x1, y1) = (px(bled.rect.right().get()), px(bled.rect.bottom().get()));
        for y in y0..y1.min(img.height()) {
            for x in x0..x1.min(img.width()) {
                let c = img.get_pixel(x, y).0;
                assert!(
                    c[0] < 40 && c[2] < 40,
                    "pixel ({x}, {y}) com o fundo em {rect:?} a {degrees} décimos: {c:?}"
                );
            }
        }
    }
}

#[test]
fn af_b6_sem_ajuste_e_o_caminho_de_sempre() {
    let image = gradient(600, 400);
    let mut album = Album::new(10.0, 100, 3.0);
    let id = album.photo("a.jpg", image.clone(), PhotoAdjust::default());
    album.frame(id, [0.55, 0.2, 0.3, 0.4], Crop::default());
    let p = &album.project;
    let frame = &p.spreads[0].frames[0];
    let rect = p.frame_rect(frame);
    let bled = bleed(&p.spec, rect);
    let crop = Crop::default();
    let out = ((120, 80), Scale { x: 1.0, y: 1.0 });
    let neutral = frame_pixels(&image, &crop, &PhotoAdjust::default(), (rect, &bled), out).unwrap();
    // O de antes da AF: a região do recorte reduzida com Lanczos3, sem mais nada.
    let r = region(&crop, rect, &bled, (600.0, 400.0));
    let src = fast_image_resize::images::ImageRef::new(
        600,
        400,
        image.as_raw(),
        fast_image_resize::PixelType::U8x4,
    )
    .unwrap();
    let mut dst =
        fast_image_resize::images::Image::new(120, 80, fast_image_resize::PixelType::U8x4);
    let options = fast_image_resize::ResizeOptions::new()
        .resize_alg(fast_image_resize::ResizeAlg::Convolution(
            fast_image_resize::FilterType::Lanczos3,
        ))
        .crop(r.x, r.y, r.width, r.height);
    fast_image_resize::Resizer::new()
        .resize(&src, &mut dst, &options)
        .unwrap();
    assert_eq!(neutral, dst.into_vec());
}

#[test]
fn af_b8_tela_igual_a_impressao() {
    let mut album = Album::new(10.0, 200, 3.0);
    let id = album.photo(
        "a.jpg",
        gradient(1200, 800),
        PhotoAdjust {
            angle_tenths: 73,
            brightness: 30,
            contrast: 40,
            bw: false,
            ..PhotoAdjust::default()
        },
    );
    album.frame(id, [0.55, 0.2, 0.3, 0.6], Crop::default());
    let print = album.render(RenderTarget::Print);
    let screen = album.render(RenderTarget::Preview { max_side: 400 });
    let reduced = image::imageops::resize(
        &print,
        screen.width(),
        screen.height(),
        image::imageops::FilterType::Triangle,
    );
    // Só o miolo do quadro (as bordas sofrem com o arredondamento de cada escala).
    let canvas = album.project.spec.canvas_rect();
    let rect = album
        .project
        .frame_rect(&album.project.spreads[0].frames[0]);
    let per = f64::from(screen.width()) / canvas.width.get();
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let px = |mm: f64| (mm * per).round() as u32;
    let (mut sum, mut n) = (0u64, 0u64);
    for y in px(rect.y.get()) + 3..px(rect.bottom().get()) - 3 {
        for x in px(rect.x.get()) + 3..px(rect.right().get()) - 3 {
            let (a, b) = (screen.get_pixel(x, y).0, reduced.get_pixel(x, y).0);
            for k in 0..3 {
                sum += u64::from(a[k].abs_diff(b[k]));
                n += 1;
            }
        }
    }
    let mean = sum as f64 / n as f64;
    assert!(mean <= 2.0, "diferença média de {mean:.2} níveis");
}

#[test]
fn af_b14_mesma_entrada_mesmos_pixels() {
    let mut album = Album::new(10.0, 100, 3.0);
    let a = album.photo(
        "a.jpg",
        gradient(600, 400),
        PhotoAdjust {
            angle_tenths: -127,
            bw: true,
            contrast: 25,
            ..PhotoAdjust::default()
        },
    );
    album.frame(a, [0.0, 0.0, 0.5, 1.0], Crop::default());
    let first = album.render(RenderTarget::Print);
    let second = album.render(RenderTarget::Print);
    assert_eq!(first.as_raw(), second.as_raw());
}

/// Seis fotos giradas e ajustadas numa lâmina 30 × 30.
fn six() -> Album {
    let mut album = Album::new(30.0, 300, 3.0);
    let rects = [
        [0.0, 0.0, 0.25, 0.5],
        [0.25, 0.0, 0.25, 0.5],
        [0.0, 0.5, 0.5, 0.5],
        [0.5, 0.0, 0.25, 0.5],
        [0.75, 0.0, 0.25, 0.5],
        [0.5, 0.5, 0.5, 0.5],
    ];
    for (k, rect) in rects.iter().enumerate() {
        let adjust = PhotoAdjust {
            angle_tenths: 15 * (i32::try_from(k).unwrap_or(0) + 1),
            brightness: 10,
            contrast: 20,
            bw: k % 2 == 0,
            ..PhotoAdjust::default()
        };
        let id = album.photo(&format!("f{k}.jpg"), gradient(3000, 2000), adjust);
        album.frame(id, *rect, Crop::default());
    }
    album
}

#[test]
fn af_b16_quadro_e_lamina_rapido() {
    let album = six();
    let p = &album.project;
    // Um quadro de meia lâmina na tela (prévia de 1600 px de largura), da imagem de tela.
    let screen = gradient(1600, 1066);
    let frame = &p.spreads[0].frames[2];
    let rect = p.frame_rect(frame);
    let bled = bleed(&p.spec, rect);
    let per = 1600.0 / p.spec.canvas_rect().width.get();
    let FrameContent::Photo { crop, .. } = &frame.content else {
        return;
    };
    let adjust = p.photos[2].adjust;
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let out = (
        (bled.rect.width.get() * per).round() as u32,
        (bled.rect.height.get() * per).round() as u32,
    );
    let mut best = Duration::MAX;
    for _ in 0..TRIES {
        let start = Instant::now();
        let pixels = frame_pixels(
            &screen,
            crop,
            &adjust,
            (rect, &bled),
            (out, Scale { x: per, y: per }),
        );
        best = best.min(start.elapsed());
        assert!(!pixels.unwrap().is_empty());
        if best <= budget(50) {
            break;
        }
    }
    assert!(best <= budget(50), "quadro na tela: {best:?}");
    let mut best = Duration::MAX;
    for _ in 0..HEAVY_TRIES {
        let start = Instant::now();
        let img = album.render(RenderTarget::Print);
        best = best.min(start.elapsed());
        assert_eq!(img.width(), 7157);
        if best <= budget(3000) {
            break;
        }
    }
    assert!(best <= budget(3000), "lâmina exportada: {best:?}");
}

#[test]
fn af_b1_a_foto_gira_no_sentido_horario() {
    // Metade esquerda vermelha, direita azul. Girada no sentido horário, a divisa vertical
    // inclina: em cima ela vai para a direita (o meio do topo fica vermelho) e embaixo para a
    // esquerda (o meio de baixo fica azul).
    let halves = RgbaImage::from_fn(800, 800, |x, _| {
        if x < 400 {
            Rgba([255, 0, 0, 255])
        } else {
            Rgba([0, 0, 255, 255])
        }
    });
    let mut album = Album::new(10.0, 100, 0.0);
    let id = album.photo("a.jpg", halves, turned(100));
    album.frame(id, [0.55, 0.2, 0.2, 0.4], Crop::default());
    let img = album.render(RenderTarget::Print);
    let p = &album.project;
    let rect = p.frame_rect(&p.spreads[0].frames[0]);
    let per = f64::from(img.width()) / p.spec.canvas_rect().width.get();
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let px = |mm: f64| (mm * per).round() as u32;
    let mid = px(rect.x.get() + rect.width.get() / 2.0);
    let top = img.get_pixel(mid, px(rect.y.get()) + 2).0;
    let bottom = img.get_pixel(mid, px(rect.bottom().get()) - 3).0;
    assert!(top[0] > 200 && top[2] < 50, "topo {top:?}");
    assert!(bottom[2] > 200 && bottom[0] < 50, "base {bottom:?}");
}
