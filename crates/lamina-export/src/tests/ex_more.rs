//! Cenários EX-B7 a EX-B12 da spec `specs/exportacao.md`.

use std::cell::RefCell;
use std::time::Instant;

use lamina_core::FrameContent;

use super::*;
use crate::preflight;
use lamina_core::testing::budget;

fn messages(album: &Album) -> Vec<String> {
    let all: Vec<usize> = (0..album.project.spreads.len()).collect();
    preflight(&album.project, &all)
        .iter()
        .map(ToString::to_string)
        .collect()
}

#[test]
fn ex_b7_foto_ampliada_demais() {
    let mut album = Album::new(spec(), 2);
    // Quadros de 300 × 200 mm atravessando a lombada, dentro da zona segura.
    let rect = [0.25, 1.0 / 6.0, 0.5, 2.0 / 3.0];
    let a = album.photo("a.jpg", solid(1500, 1000, [9, 9, 9]));
    let b = album.photo("b.jpg", solid(6000, 4000, [9, 9, 9]));
    album.photo_frame(0, a, rect);
    album.photo_frame(1, b, rect);
    assert_eq!(
        messages(&album),
        ["Lâmina 1: a foto a.jpg está ampliada 236% (127 DPI efetivo)."]
    );
}

#[test]
fn ex_b8_quadro_vazio() {
    let mut album = Album::new(spec(), 2);
    album.frame(1, FrameContent::Empty, [0.6, 0.2, 0.2, 0.2]);
    assert_eq!(messages(&album), ["Lâmina 2: quadro vazio."]);
}

#[test]
fn ex_b9_perto_do_corte() {
    let mut album = Album::new(spec(), 1);
    let c = album.photo("c.jpg", solid(6000, 4000, [9, 9, 9]));
    let d = album.photo("d.jpg", solid(6000, 4000, [9, 9, 9]));
    // c a 2 mm da borda externa esquerda; d encostado no corte de baixo.
    album.photo_frame(0, c, [2.0 / 600.0, 0.2, 0.15, 0.3]);
    album.photo_frame(0, d, [0.6, 0.7, 0.2, 0.3]);
    assert_eq!(
        messages(&album),
        ["Lâmina 1: a foto c.jpg está fora da zona segura, a 2 mm do corte."]
    );
}

#[test]
fn quadro_exatamente_na_zona_segura_nao_e_aviso() {
    // Achado com as fotos de teste: quadro a 10 mm do corte (a borda da zona segura) virava aviso.
    let mut album = Album::new(spec(), 1);
    let e = album.photo("e.jpg", solid(6000, 4000, [9, 9, 9]));
    let (x, y) = (10.0 / 600.0, 10.0 / 300.0);
    album.photo_frame(0, e, [x, y, 0.3 - x, 1.0 - 2.0 * y]);
    assert!(messages(&album).is_empty(), "{:?}", messages(&album));
}

#[test]
fn paginas_soltas_com_intervalo() {
    let s = spec().with_edges(AlbumEdge::SinglePage, AlbumEdge::SinglePage);
    let album = Album::new(s, 3);
    let dir = tempfile::tempdir().unwrap();
    let options = ExportOptions {
        pages: true,
        range: Some("2".to_owned()),
        ..ExportOptions::default()
    };
    album.export(dir.path(), &options).unwrap();
    // A lâmina 2 tem as páginas 2 e 3 do álbum (a lâmina 1 só tem a página 1).
    let mut names: Vec<String> = std::fs::read_dir(dir.path())
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    assert_eq!(names, ["pagina-02.jpg", "pagina-03.jpg"]);
}

#[test]
fn paginas_esquerda_e_direita_tem_o_mesmo_tamanho() {
    let s = AlbumSpec::new(Mm::from_cm(30.0), Mm::from_cm(30.0), Dpi::new(254).unwrap())
        .unwrap()
        .with_bleed(Mm(2.5))
        .unwrap();
    let album = Album::new(s, 1);
    let dir = tempfile::tempdir().unwrap();
    let options = ExportOptions {
        pages: true,
        ..ExportOptions::default()
    };
    let report = album.export(dir.path(), &options).unwrap();
    let sizes: Vec<(u32, u32)> = report
        .files
        .iter()
        .map(|f| image::image_dimensions(f).unwrap())
        .collect();
    assert_eq!(sizes.len(), 2);
    assert_eq!(sizes[0], sizes[1]);
}

#[test]
fn nomes_de_saida_sem_gravar_nada() {
    let album = Album::new(spec(), 3);
    let names = crate::output_names(&album.project, &ExportOptions::default()).unwrap();
    assert_eq!(names, ["lamina-01.jpg", "lamina-02.jpg", "lamina-03.jpg"]);
    let pages = ExportOptions {
        pages: true,
        range: Some("2".to_owned()),
        ..ExportOptions::default()
    };
    assert_eq!(
        crate::output_names(&album.project, &pages).unwrap(),
        ["pagina-03.jpg", "pagina-04.jpg"]
    );
}

fn tmp_files(dir: &Path) -> usize {
    std::fs::read_dir(dir)
        .unwrap()
        .filter(|e| {
            e.as_ref()
                .unwrap()
                .path()
                .extension()
                .is_some_and(|x| x == "tmp")
        })
        .count()
}

#[test]
fn ex_b10_progresso_e_arquivos_inteiros() {
    let album = Album::new(spec(), 4);
    let dir = tempfile::tempdir().unwrap();
    let calls = RefCell::new(Vec::new());
    let source = |_: &Photo, _: (u32, u32)| Err::<Arc<RgbaImage>, String>("sem fotos".to_owned());
    let progress = |done, total| calls.borrow_mut().push((done, total));
    export(
        &album.project,
        dir.path(),
        &source,
        &ExportOptions::default(),
        &progress,
    )
    .unwrap();
    assert_eq!(calls.into_inner(), [(1, 4), (2, 4), (3, 4), (4, 4)]);
    assert_eq!(tmp_files(dir.path()), 0);
}

#[test]
fn ex_b11_foto_ilegivel_no_meio() {
    let mut album = three();
    let missing = album.project.add_photo("sumiu.jpg", 600, 400);
    album.photo_frame(1, missing, [0.05, 0.2, 0.3, 0.4]);
    let dir = tempfile::tempdir().unwrap();
    let err = album
        .export(dir.path(), &ExportOptions::default())
        .unwrap_err();
    let ExportError::Stopped { done, source } = &err else {
        panic!("esperava Stopped, veio {err:?}");
    };
    assert!(matches!(**source, ExportError::Render(_)));
    // A lâmina 1 (do mesmo lote) terminou e ficou gravada; o erro diz isso.
    let first = dir.path().join("lamina-01.jpg");
    assert!(done.contains(&first) && first.exists(), "{done:?}");
    assert_eq!(
        err.to_string(),
        "Não foi possível ler a foto sumiu.jpg: arquivo sumiu."
    );
    assert!(!dir.path().join("lamina-02.jpg").exists());
    assert_eq!(tmp_files(dir.path()), 0);
}

/// Parecida com uma foto: degradê suave com granulação leve (ruído puro é o pior caso do JPEG
/// e não representa fotos de verdade).
fn photo_like(w: u32, h: u32) -> RgbaImage {
    let mut seed: u32 = 7;
    RgbaImage::from_fn(w, h, |x, y| {
        seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        let grain = (seed >> 28) as u8; // 0 a 15
        let r = u8::try_from(x * 200 / w).unwrap_or(0).saturating_add(grain);
        let g = u8::try_from(y * 200 / h).unwrap_or(0).saturating_add(grain);
        image::Rgba([r, g, 120u8.saturating_add(grain), 255])
    })
}

#[test]
fn ex_b12_rapido_o_bastante() {
    let mut album = Album::new(spec(), 10);
    // Seis fotos diferentes de 24 megapixels (auditoria do Marco 1, M5), 6 quadros por lâmina.
    let photos: Vec<_> = (0..6u32)
        .map(|n| {
            let mut image = photo_like(6000, 4000);
            image.put_pixel(n, n, image::Rgba([255, 255, 255, 255]));
            album.photo(&format!("g{n}.jpg"), image)
        })
        .collect();
    for s in 0..10 {
        for col in 0..2u32 {
            for row in 0..3u32 {
                let x = 0.03 + f64::from(col) * 0.5;
                let y = 0.04 + f64::from(row) * 0.32;
                let photo = photos[(col * 3 + row) as usize];
                album.photo_frame(s, photo, [x, y, 0.44, 0.29]);
            }
        }
    }
    let dir = tempfile::tempdir().unwrap();
    let start = Instant::now();
    album.export(dir.path(), &ExportOptions::default()).unwrap();
    let elapsed = start.elapsed();
    assert!(elapsed < budget(15_000), "{elapsed:?}");
    let first = std::fs::read(dir.path().join("lamina-01.jpg")).unwrap();
    let again = tempfile::tempdir().unwrap();
    let options = ExportOptions {
        range: Some("1".to_owned()),
        ..ExportOptions::default()
    };
    album.export(again.path(), &options).unwrap();
    assert_eq!(
        std::fs::read(again.path().join("lamina-01.jpg")).unwrap(),
        first
    );
}
