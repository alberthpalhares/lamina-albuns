//! Marcas de tela (spec RL, regra 3; spec JE, seção 4): o X do quadro vazio, a marca da foto
//! sumida e o véu das páginas inativas, só na imagem do rolo.

use std::sync::Arc;

use image::{Rgba, RgbaImage};
use lamina_core::{
    AlbumEdge, AlbumSpec, Crop, Dpi, Edges, FrameContent, Mm, Photo, Project, RelRect,
};

use crate::{
    EMPTY_GRAY, INACTIVE_GRAY, RenderTarget, ScreenMarks, draw_screen_marks, render_spread,
};

fn spec() -> AlbumSpec {
    AlbumSpec::new(Mm::from_cm(20.0), Mm::from_cm(20.0), Dpi::new(100).unwrap())
        .unwrap()
        .with_bleed(Mm(3.0))
        .unwrap()
}

/// Uma lâmina com uma foto à esquerda e um quadro vazio à direita.
fn album(edges: Edges) -> Project {
    let mut project = Project::new("teste", spec().with_edges(edges.start, edges.end));
    project.add_spread();
    project.add_spread();
    let photo = project.add_photo("a.png", 40, 30);
    let id = project.spreads[0].id;
    let content = FrameContent::Photo {
        photo,
        crop: Crop::default(),
    };
    let left = RelRect::new(0.1, 0.2, 0.3, 0.6).unwrap();
    project.add_frame(id, left, content).unwrap();
    let right = RelRect::new(0.6, 0.2, 0.3, 0.6).unwrap();
    project.add_frame(id, right, FrameContent::Empty).unwrap();
    project
}

fn red(_: &Photo, _: (u32, u32)) -> Result<Arc<RgbaImage>, String> {
    Ok(Arc::new(RgbaImage::from_pixel(
        40,
        30,
        Rgba([200, 0, 0, 255]),
    )))
}

fn preview(project: &Project) -> RgbaImage {
    render_spread(project, 0, RenderTarget::Preview { max_side: 800 }, &red).unwrap()
}

fn marks(missing: &[lamina_core::PhotoId]) -> ScreenMarks<'_> {
    ScreenMarks { missing, line: 2.0 }
}

fn luma(p: &Rgba<u8>) -> u32 {
    u32::from(p[0]) + u32::from(p[1]) + u32::from(p[2])
}

/// O meio do quadro `[x, y, w, h]` (frações do corte) em pixels da imagem.
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn at(image: &RgbaImage, project: &Project, rel: [f64; 4], fx: f64, fy: f64) -> (u32, u32) {
    let (trim, canvas) = (project.spec.trim_rect(), project.spec.canvas_rect());
    let x = trim.x.get() + (rel[0] + rel[2] * fx) * trim.width.get();
    let y = trim.y.get() + (rel[1] + rel[3] * fy) * trim.height.get();
    let px = |mm: f64, total: f64, n: u32| (mm / total * f64::from(n)).floor() as u32;
    (
        px(x, canvas.width.get(), image.width()),
        px(y, canvas.height.get(), image.height()),
    )
}

#[test]
fn quadro_vazio_fica_cinza_com_as_diagonais_mais_claras() {
    let project = album(Edges::default());
    let image = draw_screen_marks(preview(&project), &project, 0, &marks(&[])).unwrap();
    let quad = [0.6, 0.2, 0.3, 0.6];
    let (cx, cy) = at(&image, &project, quad, 0.5, 0.5);
    let (gx, gy) = at(&image, &project, quad, 0.5, 0.15);
    let gray = Rgba([EMPTY_GRAY[0], EMPTY_GRAY[1], EMPTY_GRAY[2], 255]);
    assert_eq!(
        *image.get_pixel(gx, gy),
        gray,
        "fora das diagonais, o cinza"
    );
    assert!(
        luma(image.get_pixel(cx, cy)) > luma(&gray),
        "o cruzamento do X é mais claro"
    );
    // A impressão continua sem nada no quadro vazio: a cor do fundo.
    let plain = preview(&project);
    assert_eq!(*plain.get_pixel(cx, cy), Rgba([255, 255, 255, 255]));
}

#[test]
fn foto_sumida_fica_escurecida_e_a_presente_nao_muda() {
    let project = album(Edges::default());
    let photo = project.photos[0].id;
    let plain = preview(&project);
    let shown = draw_screen_marks(plain.clone(), &project, 0, &marks(&[])).unwrap();
    let quad = [0.1, 0.2, 0.3, 0.6];
    let (x, y) = at(&plain, &project, quad, 0.1, 0.1);
    assert_eq!(shown.get_pixel(x, y), plain.get_pixel(x, y));
    let gone = draw_screen_marks(plain.clone(), &project, 0, &marks(&[photo])).unwrap();
    assert!(luma(gone.get_pixel(x, y)) < luma(plain.get_pixel(x, y)));
}

#[test]
fn pagina_inativa_ganha_o_veu_e_o_resto_fica_igual() {
    let edges = Edges::new(AlbumEdge::SinglePage, AlbumEdge::FullSpread);
    let project = album(edges);
    let plain = preview(&project);
    let image = draw_screen_marks(plain.clone(), &project, 0, &marks(&[])).unwrap();
    let veil = Rgba([INACTIVE_GRAY[0], INACTIVE_GRAY[1], INACTIVE_GRAY[2], 255]);
    assert_eq!(*image.get_pixel(3, plain.height() / 2), veil);
    // Fora dos quadros e da página inativa, nada muda.
    let (x, y) = (plain.width() - 3, 3);
    assert_eq!(image.get_pixel(x, y), plain.get_pixel(x, y));
    // A segunda lâmina não tem página inativa nem quadros: a imagem é a mesma.
    let second = render_spread(&project, 1, RenderTarget::Preview { max_side: 800 }, &red);
    let second = second.unwrap();
    let marked = draw_screen_marks(second.clone(), &project, 1, &marks(&[])).unwrap();
    assert_eq!(marked, second);
}

#[test]
fn lamina_que_nao_existe_e_erro() {
    let project = album(Edges::default());
    let result = draw_screen_marks(preview(&project), &project, 9, &marks(&[]));
    assert!(matches!(
        result,
        Err(crate::RenderError::NoSpread {
            number: 10,
            count: 2
        })
    ));
}
