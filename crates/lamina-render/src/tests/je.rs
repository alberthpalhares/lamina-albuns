//! Cenário JE-B2 da spec `janela-enxuta`: o X do quadro vazio nas imagens de tela, nunca na de
//! impressão.

use std::sync::Arc;

use image::{Rgba, RgbaImage};
use lamina_core::{AlbumSpec, Crop, Dpi, FrameContent, Mm, Photo, Project, RelRect, Rgb, SpreadId};

use crate::{EMPTY_GRAY, RenderTarget, draw_empty_marks, render_spread};

const PAGE: Rgb = Rgb {
    r: 240,
    g: 236,
    b: 228,
};

/// Uma lâmina com duas fotos e um quadro vazio quadrado, e a lâmina dele.
fn album() -> (Project, SpreadId) {
    let spec = AlbumSpec::new(Mm::from_cm(30.0), Mm::from_cm(30.0), Dpi::new(300).unwrap())
        .unwrap()
        .with_bleed(Mm(3.0))
        .unwrap();
    let mut project = Project::new("teste", spec);
    let spread = project.add_spread();
    project.spreads[0].background = PAGE;
    for (i, x) in [0.05, 0.3].into_iter().enumerate() {
        let photo = project.add_photo(format!("f{i}.jpg"), 600, 400);
        let rect = RelRect::new(x, 0.2, 0.2, 0.4).unwrap();
        let content = FrameContent::Photo {
            photo,
            crop: Crop::default(),
        };
        project.add_frame(spread, rect, content).unwrap();
    }
    let rect = RelRect::new(0.6, 0.2, 0.3, 0.6).unwrap();
    project
        .add_frame(spread, rect, FrameContent::Empty)
        .unwrap();
    (project, spread)
}

fn render(project: &Project, target: RenderTarget) -> RgbaImage {
    let red = Arc::new(RgbaImage::from_pixel(600, 400, Rgba([200, 50, 50, 255])));
    let source = move |_: &Photo, _: (u32, u32)| Ok::<_, String>(red.clone());
    render_spread(project, 0, target, &source).unwrap()
}

/// O centro do quadro vazio e um ponto a um quarto da largura dele, longe das diagonais.
fn points(project: &Project, image: &RgbaImage) -> ((u32, u32), (u32, u32)) {
    let frame = &project.spreads[0].frames[2];
    let r = project.frame_rect(frame);
    let canvas = project.spec.canvas_rect();
    let s = f64::from(image.width()) / canvas.width.get();
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let px = |mm: f64| (mm * s).round() as u32;
    let (x0, y0, x1, y1) = (
        px(r.x.get()),
        px(r.y.get()),
        px(r.right().get()),
        px(r.bottom().get()),
    );
    let center = ((x0 + x1) / 2, (y0 + y1) / 2);
    let off = (x0 + (x1 - x0) / 4, (y0 + y1) / 2);
    (center, off)
}

fn rgb(image: &RgbaImage, at: (u32, u32)) -> [u8; 3] {
    let p = image.get_pixel(at.0, at.1).0;
    [p[0], p[1], p[2]]
}

#[test]
fn je_b2_o_x_nas_imagens_de_tela_e_nunca_na_de_impressao() {
    let (project, _) = album();
    let mut screen = render(&project, RenderTarget::Preview { max_side: 1200 });
    let before = screen.clone();
    draw_empty_marks(&mut screen, &project, 0, 1.5);
    let (center, off) = points(&project, &screen);
    assert_eq!(rgb(&screen, off), EMPTY_GRAY, "o quadro vazio é cinza");
    let mark = rgb(&screen, center);
    assert!(
        mark.iter().zip(EMPTY_GRAY).all(|(m, g)| *m > g + 20),
        "as diagonais são mais claras que o cinza: {mark:?}"
    );
    assert!(
        screen.pixels().all(|p| p.0[3] == 255),
        "a imagem continua opaca"
    );
    // Fora do quadro vazio, nada muda.
    let outside = (5, 5);
    assert_eq!(
        screen.get_pixel(outside.0, outside.1),
        before.get_pixel(outside.0, outside.1)
    );

    let print = render(&project, RenderTarget::Print);
    let (center, off) = points(&project, &print);
    let page = [PAGE.r, PAGE.g, PAGE.b];
    assert_eq!(rgb(&print, center), page, "a impressão não tem o X");
    assert_eq!(rgb(&print, off), page, "nem o cinza");
}
