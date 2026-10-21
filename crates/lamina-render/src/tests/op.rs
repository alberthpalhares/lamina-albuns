//! Cenários da spec `opacidade-da-foto` no renderizador: a mistura com o fundo e com outra foto,
//! a borda opaca e o texto intacto por cima (OP-B3 a OP-B5).

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use image::{Rgba, RgbaImage};
use lamina_core::{
    AlbumSpec, Border, Crop, Dpi, FrameContent, Mm, Opacity, Photo, PhotoId, Project, RelRect, Rgb,
    TextBox,
};

use crate::style::styled;
use crate::{RenderTarget, render_spread};

const PHOTO_A: [u8; 3] = [200, 100, 40];
const PHOTO_B: [u8; 3] = [0, 200, 100];
const BACK: Rgb = Rgb {
    r: 20,
    g: 60,
    b: 240,
};

/// Uma lâmina de páginas de 10 cm a 300 DPI, com o fundo azul e fotos de uma cor só.
struct Album {
    project: Project,
    images: HashMap<PathBuf, Arc<RgbaImage>>,
}

impl Album {
    fn new() -> Self {
        let spec =
            AlbumSpec::new(Mm::from_cm(10.0), Mm::from_cm(10.0), Dpi::new(300).unwrap()).unwrap();
        let mut project = Project::new("teste", spec);
        project.add_spread();
        project.spreads[0].background = BACK;
        Album {
            project,
            images: HashMap::new(),
        }
    }

    /// Uma foto de cor `c` no retângulo `rect` (frações do corte), com a opacidade `percent`.
    fn photo(&mut self, name: &str, c: [u8; 3], rect: [f64; 4], percent: u8) -> PhotoId {
        let image = RgbaImage::from_pixel(300, 300, Rgba([c[0], c[1], c[2], 255]));
        let id = self.project.add_photo(name, 300, 300);
        self.images.insert(PathBuf::from(name), Arc::new(image));
        let spread = self.project.spreads[0].id;
        let rect = RelRect::new(rect[0], rect[1], rect[2], rect[3]).unwrap();
        let content = FrameContent::Photo {
            photo: id,
            crop: Crop::default(),
        };
        self.project.add_frame(spread, rect, content).unwrap();
        if let Some(f) = self.project.spreads[0].frames.last_mut() {
            f.opacity = Opacity::new(percent).unwrap();
        }
        id
    }

    fn render(&self) -> RgbaImage {
        let source = |p: &Photo, _: (u32, u32)| {
            self.images
                .get(&p.path)
                .cloned()
                .ok_or_else(|| "arquivo sumiu".to_owned())
        };
        render_spread(&self.project, 0, RenderTarget::Print, &source).unwrap()
    }

    /// O pixel no ponto (fração da largura e da altura do corte).
    fn at(&self, img: &RgbaImage, x: f64, y: f64) -> [u8; 3] {
        let trim = self.project.spec.trim_rect();
        let s = f64::from(img.width()) / self.project.spec.canvas_rect().width.get();
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let px = |mm: f64| (mm * s) as u32;
        let p = img
            .get_pixel(
                px(trim.x.get() + x * trim.width.get()),
                px(trim.y.get() + y * trim.height.get()),
            )
            .0;
        [p[0], p[1], p[2]]
    }
}

fn near(a: [u8; 3], b: [u8; 3]) -> bool {
    a.iter().zip(b).all(|(x, y)| x.abs_diff(y) <= 1)
}

fn opaque(img: &RgbaImage) -> bool {
    img.pixels().all(|p| p.0[3] == 255)
}

#[test]
fn op_b3_a_mistura_com_o_fundo() {
    let mut half = Album::new();
    half.photo("a.jpg", PHOTO_A, [0.2, 0.2, 0.6, 0.6], 50);
    let img = half.render();
    let got = half.at(&img, 0.5, 0.5);
    assert!(near(got, [110, 80, 140]), "50%: {got:?}");
    assert!(opaque(&img));
    assert!(styled(&half.project.spreads[0].frames[0]));

    let mut full = Album::new();
    full.photo("a.jpg", PHOTO_A, [0.2, 0.2, 0.6, 0.6], 100);
    assert!(
        !styled(&full.project.spreads[0].frames[0]),
        "100% vai pelo caminho de antes (paste)"
    );
    let img = full.render();
    assert_eq!(full.at(&img, 0.5, 0.5), PHOTO_A);
    assert!(opaque(&img));

    let mut none = Album::new();
    none.photo("a.jpg", PHOTO_A, [0.2, 0.2, 0.6, 0.6], 0);
    let img = none.render();
    assert_eq!(none.at(&img, 0.5, 0.5), [BACK.r, BACK.g, BACK.b]);
    assert!(opaque(&img));
}

#[test]
fn op_b4_a_borda_continua_opaca() {
    let mut album = Album::new();
    album.photo("a.jpg", PHOTO_A, [0.2, 0.2, 0.6, 0.6], 50);
    let frame = &mut album.project.spreads[0].frames[0];
    frame.border = Border {
        width: Mm(2.0),
        color: Rgb::WHITE,
    };
    frame.radius = Mm(5.0);
    let img = album.render();
    // A borda de 2 mm: 1 mm para dentro do quadro (o corte tem 200 mm de largura).
    let edge = 0.2 + 1.0 / 200.0;
    assert_eq!(album.at(&img, edge, 0.5), [255, 255, 255], "borda opaca");
    assert!(
        near(album.at(&img, 0.5, 0.5), [110, 80, 140]),
        "o miolo mistura"
    );
    assert_eq!(
        album.at(&img, 0.2 + 0.0005, 0.2 + 0.0005),
        [BACK.r, BACK.g, BACK.b],
        "o canto arredondado é o fundo"
    );
    assert!(opaque(&img));
}

#[test]
fn op_b5_por_cima_de_outra_foto_e_por_baixo_do_texto() {
    let build = |percent: u8| {
        let mut album = Album::new();
        album.photo("b.jpg", PHOTO_B, [0.1, 0.1, 0.4, 0.8], 100);
        album.photo("a.jpg", PHOTO_A, [0.3, 0.1, 0.6, 0.8], percent);
        let spread = album.project.spreads[0].id;
        let text = TextBox {
            text: "Lamina".to_owned(),
            size_pt: 40.0,
            ..TextBox::new(Rgb::BLACK)
        };
        let rect = RelRect::new(0.35, 0.4, 0.5, 0.2).unwrap();
        album
            .project
            .add_frame(spread, rect, FrameContent::Text(text))
            .unwrap();
        album
    };
    let album = build(50);
    let img = album.render();
    let over_b = album.at(&img, 0.4, 0.2);
    assert!(near(over_b, [100, 150, 70]), "A sobre B: {over_b:?}");
    let over_back = album.at(&img, 0.8, 0.2);
    assert!(
        near(over_back, [110, 80, 140]),
        "A sobre o fundo: {over_back:?}"
    );
    // Os pixels do texto inteiramente pretos são os mesmos da lâmina com A a 100%.
    let solid = build(100).render();
    let black = |p: &Rgba<u8>| p.0[..3] == [0, 0, 0];
    let ink: Vec<(u32, u32)> = solid
        .enumerate_pixels()
        .filter(|(_, _, p)| black(p))
        .map(|(x, y, _)| (x, y))
        .collect();
    assert!(!ink.is_empty(), "o texto aparece");
    for (x, y) in ink {
        assert_eq!(
            img.get_pixel(x, y),
            solid.get_pixel(x, y),
            "texto em ({x}, {y})"
        );
    }
}
