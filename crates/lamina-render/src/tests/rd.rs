//! Cenários da spec `specs/renderizacao.md` (RD-B1 a RD-B9, RD-B11 e RD-B12). O RD-B10 (foto
//! girada pelo EXIF) fica na `lamina-media`, que lê os arquivos.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

use image::{Rgba, RgbaImage};
use lamina_core::{
    AlbumSpec, Crop, Dpi, FrameContent, Mm, Photo, PhotoId, Project, RelRect, Rgb, SpreadId,
};

use crate::{RenderError, RenderTarget, render_spread};

const RED: [u8; 3] = [255, 0, 0];
const GREEN: [u8; 3] = [0, 255, 0];
const BLUE: [u8; 3] = [0, 0, 255];
const YELLOW: [u8; 3] = [255, 255, 0];
const WHITE: [u8; 3] = [255, 255, 255];

fn spec(bleed_mm: f64) -> AlbumSpec {
    AlbumSpec::new(Mm::from_cm(30.0), Mm::from_cm(30.0), Dpi::new(300).unwrap())
        .unwrap()
        .with_bleed(Mm(bleed_mm))
        .unwrap()
}

/// Projeto de teste: fotos em memória por nome, e uma lâmina.
struct Album {
    project: Project,
    spread: SpreadId,
    images: HashMap<PathBuf, Arc<RgbaImage>>,
}

impl Album {
    fn new(bleed_mm: f64) -> Self {
        let mut project = Project::new("teste", spec(bleed_mm));
        let spread = project.add_spread();
        Album {
            project,
            spread,
            images: HashMap::new(),
        }
    }

    fn photo(&mut self, name: &str, image: RgbaImage) -> PhotoId {
        let id = self.project.add_photo(name, image.width(), image.height());
        self.images.insert(PathBuf::from(name), Arc::new(image));
        id
    }

    fn frame(&mut self, photo: PhotoId, rect: [f64; 4], crop: Crop) {
        let rect = RelRect::new(rect[0], rect[1], rect[2], rect[3]).unwrap();
        let content = FrameContent::Photo { photo, crop };
        self.project.add_frame(self.spread, rect, content).unwrap();
    }

    fn render(&self, target: RenderTarget) -> Result<RgbaImage, RenderError> {
        let source = |p: &Photo, _: (u32, u32)| {
            self.images
                .get(&p.path)
                .cloned()
                .ok_or_else(|| "arquivo sumiu".to_owned())
        };
        render_spread(&self.project, 0, target, &source)
    }
}

/// Imagem com os quatro quadrantes em cores diferentes.
fn quadrants(w: u32, h: u32) -> RgbaImage {
    RgbaImage::from_fn(w, h, |x, y| {
        let c = match (x < w / 2, y < h / 2) {
            (true, true) => RED,
            (false, true) => GREEN,
            (true, false) => BLUE,
            (false, false) => YELLOW,
        };
        Rgba([c[0], c[1], c[2], 255])
    })
}

/// Faixas verticais vermelha, verde e azul, um terço cada.
fn stripes(w: u32, h: u32) -> RgbaImage {
    RgbaImage::from_fn(w, h, |x, _| {
        let c = [RED, GREEN, BLUE][(x * 3 / w) as usize];
        Rgba([c[0], c[1], c[2], 255])
    })
}

fn solid(w: u32, h: u32, c: [u8; 3]) -> RgbaImage {
    RgbaImage::from_pixel(w, h, Rgba([c[0], c[1], c[2], 255]))
}

/// Pixel no ponto (fx, fy) do quadro `rect` (relativo ao corte), numa impressão com sangria.
fn at(img: &RgbaImage, bleed_mm: f64, rect: [f64; 4], fx: f64, fy: f64) -> [u8; 3] {
    let (trim_w, trim_h) = (600.0, 300.0);
    let px_per_mm = f64::from(img.width()) / (trim_w + 2.0 * bleed_mm);
    let x = (bleed_mm + (rect[0] + rect[2] * fx) * trim_w) * px_per_mm;
    let y = (bleed_mm + (rect[1] + rect[3] * fy) * trim_h) * px_per_mm;
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let p = img.get_pixel(x as u32, y as u32);
    [p[0], p[1], p[2]]
}

fn close(a: [u8; 3], b: [u8; 3], tolerance: u8) -> bool {
    a.iter().zip(b).all(|(x, y)| x.abs_diff(y) <= tolerance)
}

#[test]
fn rd_b1_tamanho_da_impressao() {
    let img = Album::new(3.0).render(RenderTarget::Print).unwrap();
    assert_eq!(img.dimensions(), (7157, 3614));
}

#[test]
fn rd_b2_fundo_em_toda_a_tela() {
    let mut album = Album::new(3.0);
    album.project.spread_mut(album.spread).unwrap().background = Rgb {
        r: 10,
        g: 20,
        b: 30,
    };
    let img = album.render(RenderTarget::Print).unwrap();
    assert!(img.pixels().all(|p| p.0 == [10, 20, 30, 255]));
}

/// Quadro 3:2 na página direita: 180 × 120 mm.
const FRAME_32: [f64; 4] = [0.55, 0.2, 0.3, 0.4];
/// Quadro quadrado na página direita: 120 × 120 mm.
const FRAME_SQ: [f64; 4] = [0.6, 0.2, 0.2, 0.4];

#[test]
fn rd_b3_foto_inteira_no_lugar_certo() {
    let mut album = Album::new(3.0);
    let photo = album.photo("q.jpg", quadrants(600, 400));
    album.frame(photo, FRAME_32, Crop::default());
    let img = album.render(RenderTarget::Print).unwrap();
    for (fx, fy, color) in [
        (0.25, 0.25, RED),
        (0.75, 0.25, GREEN),
        (0.25, 0.75, BLUE),
        (0.75, 0.75, YELLOW),
    ] {
        let got = at(&img, 3.0, FRAME_32, fx, fy);
        assert!(close(got, color, 2), "({fx}, {fy}): {got:?}");
    }
}

#[test]
fn rd_b4_recorte_centralizado() {
    let mut album = Album::new(3.0);
    let photo = album.photo("f.jpg", stripes(600, 400));
    album.frame(photo, FRAME_SQ, Crop::default());
    let img = album.render(RenderTarget::Print).unwrap();
    // Aparece o miolo 400 × 400 (x de 100 a 500): 1/4 vermelho, 1/2 verde, 1/4 azul.
    for (fx, color) in [(0.1, RED), (0.5, GREEN), (0.9, BLUE)] {
        let got = at(&img, 3.0, FRAME_SQ, fx, 0.5);
        assert!(close(got, color, 2), "{fx}: {got:?}");
    }
}

#[test]
fn rd_b5_zoom_e_centro_do_recorte() {
    let mut album = Album::new(3.0);
    let photo = album.photo("q.jpg", quadrants(600, 400));
    let crop = Crop {
        zoom: 2.0,
        center_x: 0.25,
        center_y: 0.25,
    };
    album.frame(photo, FRAME_32, crop);
    let img = album.render(RenderTarget::Print).unwrap();
    for (fx, fy) in [(0.1, 0.1), (0.9, 0.1), (0.1, 0.9), (0.9, 0.9), (0.5, 0.5)] {
        let got = at(&img, 3.0, FRAME_32, fx, fy);
        assert!(close(got, RED, 2), "({fx}, {fy}): {got:?}");
    }
}

#[test]
fn rd_b6_sangria_nos_quadros_que_encostam_no_corte() {
    let mut album = Album::new(3.0);
    let photo = album.photo("p.jpg", solid(3000, 2000, YELLOW));
    album.frame(photo, [0.0, 0.0, 0.5, 1.0], Crop::default());
    let img = album.render(RenderTarget::Print).unwrap();
    let (w, h) = img.dimensions();
    let px = |x: u32, y: u32| {
        let p = img.get_pixel(x, y);
        [p[0], p[1], p[2]]
    };
    for (x, y) in [(1, 1), (1, h / 2), (1, h - 2), (w / 4, 1), (w / 4, h - 2)] {
        assert_eq!(px(x, y), YELLOW, "sangria da página esquerda em ({x}, {y})");
    }
    for (x, y) in [(w - 2, h / 2), (w * 3 / 4, 1)] {
        assert_eq!(px(x, y), WHITE, "sangria da página direita em ({x}, {y})");
    }
}

#[test]
fn sb_b12_a_foto_vai_ate_a_sangria_nas_celulas_de_borda() {
    let mut album = Album::new(3.0);
    let photo = album.photo("p.jpg", solid(3000, 2000, YELLOW));
    // Grade 2 × 2 sem borda na página esquerda: cada célula toca dois lados do corte, e a de baixo
    // à direita toca só o de baixo, porque a página da direita fica vazia.
    let gap = 8.0 * 25.4 / 72.0;
    let (w, h) = (0.25 - gap / 1200.0, 0.5 - gap / 600.0);
    album.frame(photo, [0.0, 0.0, w, h], Crop::default());
    album.frame(photo, [0.0, 1.0 - h, w, h], Crop::default());
    album.frame(photo, [0.5 - w, 1.0 - h, w, h], Crop::default());
    let img = album.render(RenderTarget::Print).unwrap();
    let (iw, ih) = img.dimensions();
    let px = |x: u32, y: u32| {
        let p = img.get_pixel(x, y);
        [p[0], p[1], p[2]]
    };
    for (x, y) in [(1, 1), (1, ih - 2), (iw * 3 / 8, ih - 2)] {
        assert_eq!(px(x, y), YELLOW, "sangria em ({x}, {y})");
    }
    assert_eq!(px(iw * 3 / 8, 1), WHITE, "sem quadro em cima à direita");
}

#[test]
fn rd_b7_quadro_nao_vaza() {
    let mut album = Album::new(0.0);
    let photo = album.photo("r.jpg", solid(300, 200, RED));
    // Dois quadros 3:2 de 120 × 80 mm com 2,822 mm entre eles.
    let gap = 8.0 * 25.4 / 72.0;
    let (w, h) = (120.0 / 600.0, 80.0 / 300.0);
    let second_x = 0.1 + w + gap / 600.0;
    album.frame(photo, [0.1, 0.2, w, h], Crop::default());
    album.frame(photo, [second_x, 0.2, w, h], Crop::default());
    let img = album.render(RenderTarget::Print).unwrap();
    let middle = [0.1 + w, 0.2, gap / 600.0, h];
    assert_eq!(
        at(&img, 0.0, middle, 0.5, 0.5),
        WHITE,
        "o espaço entre os quadros vazou"
    );
    assert!(close(at(&img, 0.0, [0.1, 0.2, w, h], 0.5, 0.5), RED, 2));
}

#[cfg(test)]
#[path = "rd_more.rs"]
mod more;
