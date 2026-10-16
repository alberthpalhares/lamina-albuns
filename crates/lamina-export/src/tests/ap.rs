//! Cenários da spec `aparencia` que olham o arquivo exportado: AP-B3 a AP-B6 e AP-B9.

use image::RgbaImage;
use lamina_core::{Border, Mm, PageSide, Project, Rect, Rgb};

use super::ex::{Album, solid, spec};
use crate::ExportOptions;

const RED: [u8; 3] = [200, 50, 50];
const CREAM: Rgb = Rgb {
    r: 245,
    g: 240,
    b: 230,
};

/// Um quadro de 100 × 66 mm no meio da página esquerda.
fn album() -> Album {
    let mut album = Album::new(spec(), 1);
    let p = album.photo("f.jpg", solid(900, 600, RED));
    album.photo_frame(0, p, [0.1, 0.3, 100.0 / 600.0, 66.0 / 300.0]);
    album
}

fn export(album: &Album, pages: bool) -> Vec<RgbaImage> {
    let dir = tempfile::tempdir().unwrap();
    let options = ExportOptions {
        pages,
        ..ExportOptions::default()
    };
    let report = album.export(dir.path(), &options).unwrap();
    report
        .files
        .iter()
        .map(|f| image::open(f).unwrap().into_rgba8())
        .collect()
}

fn near(img: &RgbaImage, x: u32, y: u32, c: [u8; 3]) -> bool {
    let p = img.get_pixel(x, y).0;
    (0..3).all(|i| p[i].abs_diff(c[i]) <= 8)
}

fn rgb(c: Rgb) -> [u8; 3] {
    [c.r, c.g, c.b]
}

/// O quadro 0 em pixels de impressão: (x0, y0, x1, y1).
fn frame_px(project: &Project) -> (u32, u32, u32, u32) {
    let r: Rect = project.frame_rect(&project.spreads[0].frames[0]);
    let s = f64::from(project.spec.dpi().get()) / 25.4;
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let px = |mm: f64| (mm * s).round() as u32;
    (
        px(r.x.get()),
        px(r.y.get()),
        px(r.right().get()),
        px(r.bottom().get()),
    )
}

#[test]
fn ap_b3_fundo_da_lamina() {
    let mut album = album();
    album.project.spreads[0].background = Rgb {
        r: 30,
        g: 30,
        b: 30,
    };
    let img = &export(&album, false)[0];
    assert!(near(img, 4, 4, [30, 30, 30]), "na sangria");
    assert!(
        near(img, img.width() - 100, img.height() / 2, [30, 30, 30]),
        "sem foto"
    );
}

#[test]
fn ap_b4_fundo_por_pagina() {
    let mut album = album();
    album.project.spreads[0].background = CREAM;
    album.project.spreads[0].background_right = Some(Rgb::BLACK);
    let whole = &export(&album, false)[0];
    let (w, h) = (whole.width(), whole.height());
    assert!(near(whole, 20, h - 20, rgb(CREAM)) && near(whole, w / 2 - 6, h - 20, rgb(CREAM)));
    assert!(near(whole, w / 2 + 6, h - 20, [0, 0, 0]) && near(whole, w - 20, h - 20, [0, 0, 0]));
    let pages = export(&album, true);
    let (left, right) = (&pages[0], &pages[1]);
    assert!(
        near(left, left.width() - 3, 10, rgb(CREAM)),
        "sangria da lombada da esquerda"
    );
    assert!(
        near(right, 3, 10, [0, 0, 0]),
        "sangria da lombada da direita"
    );
}

#[test]
fn ap_b5_borda() {
    let mut album = album();
    let rect = album.project.spreads[0].frames[0].rect;
    album.project.spreads[0].background = Rgb::BLACK;
    album.project.spreads[0].frames[0].border = Border {
        width: Mm(2.0),
        color: Rgb::WHITE,
    };
    let img = &export(&album, false)[0];
    let (x0, y0, x1, _) = frame_px(&album.project);
    let mid = y0 + 100;
    // 2 mm a 300 DPI = 24 px de borda branca; depois, a foto.
    assert!(near(img, x0 + 2, mid, [255, 255, 255]) && near(img, x0 + 20, mid, [255, 255, 255]));
    assert!(
        near(img, x0 + 32, mid, RED),
        "a foto continua depois da borda"
    );
    assert!(near(img, x1 - 3, mid, [255, 255, 255]));
    assert!(near(img, x0 - 4, mid, [0, 0, 0]), "fora do quadro, o fundo");
    assert_eq!(
        album.project.spreads[0].frames[0].rect, rect,
        "o quadro não muda"
    );
}

#[test]
fn ap_b6_cantos_arredondados() {
    let mut album = album();
    album.project.spreads[0].background = Rgb::BLACK;
    album.project.spreads[0].frames[0].radius = Mm(5.0);
    let img = &export(&album, false)[0];
    let (x0, y0, x1, y1) = frame_px(&album.project);
    assert!(near(img, x0 + 2, y0 + 2, [0, 0, 0]), "o canto é do fundo");
    assert!(near(img, (x0 + x1) / 2, (y0 + y1) / 2, RED));
    assert!(
        near(img, (x0 + x1) / 2, y0 + 3, RED),
        "o meio da beira é foto"
    );
    album.project.spreads[0].frames[0].radius = Mm(100.0);
    let img = &export(&album, false)[0];
    assert!(
        near(img, (x0 + x1) / 2, (y0 + y1) / 2, RED),
        "raio limitado: o centro é foto"
    );
    assert!(near(img, x0 + 10, y0 + 10, [0, 0, 0]));
}

/// O JSON de um projeto como a versão 2 do formato gravava.
fn as_v2(project: &Project) -> String {
    let mut v: serde_json::Value = serde_json::from_str(&project.to_json().unwrap()).unwrap();
    v["schema_version"] = 2.into();
    let o = v.as_object_mut().unwrap();
    o.remove("gap");
    for s in o["spreads"].as_array_mut().unwrap() {
        let s = s.as_object_mut().unwrap();
        s.remove("background_right");
        s.remove("gap");
        for f in s["frames"].as_array_mut().unwrap() {
            let f = f.as_object_mut().unwrap();
            f.remove("border");
            f.remove("radius");
        }
    }
    v.to_string()
}

#[test]
fn ap_b9_projeto_da_versao_2() {
    let album = album();
    let opened = Project::from_json(&as_v2(&album.project)).unwrap();
    assert_eq!(opened.schema_version, 3);
    assert_eq!(
        opened, album.project,
        "padrões: sem borda, sem cantos, mesmo fundo, 8 pt"
    );
    assert_eq!(
        opened.spreads[0].page_background(PageSide::Right),
        Rgb::WHITE
    );
    let reopened = Album {
        project: opened,
        images: album.images.clone(),
    };
    let dir = (tempfile::tempdir().unwrap(), tempfile::tempdir().unwrap());
    let a = album
        .export(dir.0.path(), &ExportOptions::default())
        .unwrap();
    let b = reopened
        .export(dir.1.path(), &ExportOptions::default())
        .unwrap();
    assert_eq!(
        std::fs::read(&a.files[0]).unwrap(),
        std::fs::read(&b.files[0]).unwrap()
    );
}
