//! Cenários da spec `specs/exportar-pela-janela.md` na exportação: EI-B4 (limpar a saída ao
//! substituir) e EI-B9 (foto repetida lida uma vez).

use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};

use lamina_core::{AlbumSpec, Dpi, Mm, Photo};

use super::ex::{Album, solid};
use crate::{ExportOptions, export};

/// Página de 5 × 5 cm a 100 DPI: lâminas pequenas, exportação rápida.
fn small() -> AlbumSpec {
    AlbumSpec::new(Mm::from_cm(5.0), Mm::from_cm(5.0), Dpi::new(100).unwrap()).unwrap()
}

fn album(spreads: usize) -> Album {
    let mut album = Album::new(small(), spreads);
    for s in 0..spreads {
        let p = album.photo(&format!("f{s}.jpg"), solid(60, 40, [200, 50, 50]));
        album.photo_frame(s, p, [0.55, 0.2, 0.3, 0.4]);
    }
    album
}

fn names(dir: &Path) -> Vec<String> {
    let mut out: Vec<String> = std::fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    out.sort();
    out
}

#[test]
fn ei_b4_substituir_apaga_so_os_arquivos_que_sobraram_com_o_nosso_nome() {
    let dir = tempfile::tempdir().unwrap();
    for n in 1..=10 {
        std::fs::write(dir.path().join(format!("lamina-{n:02}.jpg")), b"antigo").unwrap();
    }
    std::fs::write(dir.path().join("pagina-03.jpg"), b"antigo").unwrap();
    std::fs::write(dir.path().join("notas.txt"), b"minhas notas").unwrap();
    let options = ExportOptions {
        overwrite: true,
        ..ExportOptions::default()
    };
    let report = album(8).export(dir.path(), &options).unwrap();

    let mut expected: Vec<String> = (1..=8).map(|n| format!("lamina-{n:02}.jpg")).collect();
    expected.push("notas.txt".into());
    assert_eq!(names(dir.path()), expected);
    let removed: Vec<String> = report
        .removed
        .iter()
        .map(|p| p.file_name().unwrap().to_string_lossy().into_owned())
        .collect();
    assert_eq!(removed, ["lamina-09.jpg", "lamina-10.jpg", "pagina-03.jpg"]);
    for file in &report.files {
        // Um JPEG novo de verdade (os antigos eram só texto): uma lâmina, mais larga que alta.
        let (w, h) = image::image_dimensions(file).unwrap();
        assert!(w > h, "{file:?}: {w} × {h}");
    }
    assert_eq!(
        std::fs::read(dir.path().join("notas.txt")).unwrap(),
        b"minhas notas"
    );
}

#[test]
fn ei_b4_sem_substituir_nada_e_apagado() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("lamina-09.jpg"), b"antigo").unwrap();
    let report = album(2)
        .export(dir.path(), &ExportOptions::default())
        .unwrap();
    assert!(report.removed.is_empty());
    assert_eq!(
        names(dir.path()),
        ["lamina-01.jpg", "lamina-02.jpg", "lamina-09.jpg"]
    );
}

/// Exporta contando quantas vezes a fonte é chamada.
fn export_counting(album: &Album, dir: &Path) -> (usize, Vec<u8>) {
    let calls = AtomicUsize::new(0);
    let source = |p: &Photo, _: (u32, u32)| {
        calls.fetch_add(1, Ordering::SeqCst);
        album
            .images
            .get(&p.path)
            .cloned()
            .ok_or_else(|| "arquivo sumiu".to_owned())
    };
    let report = export(
        &album.project,
        dir,
        &source,
        &ExportOptions::default(),
        &|_, _| {},
    )
    .unwrap();
    (
        calls.load(Ordering::SeqCst),
        std::fs::read(&report.files[0]).unwrap(),
    )
}

#[test]
fn ei_b9_foto_repetida_e_lida_uma_vez_e_o_arquivo_nao_muda() {
    let rects = [
        [0.05, 0.1, 0.25, 0.3],
        [0.35, 0.1, 0.25, 0.3],
        [0.65, 0.1, 0.25, 0.3],
    ];
    let image = solid(60, 40, [30, 120, 200]);
    let mut repeated = Album::new(small(), 1);
    let same = repeated.photo("mesma.jpg", image.clone());
    let mut distinct = Album::new(small(), 1);
    for (i, rect) in rects.iter().enumerate() {
        repeated.photo_frame(0, same, *rect);
        let own = distinct.photo(&format!("copia{i}.jpg"), image.clone());
        distinct.photo_frame(0, own, *rect);
    }
    let (a, b) = (tempfile::tempdir().unwrap(), tempfile::tempdir().unwrap());
    let (calls, bytes) = export_counting(&repeated, a.path());
    let (calls_distinct, bytes_distinct) = export_counting(&distinct, b.path());
    assert_eq!(calls, 1, "a foto repetida foi lida {calls} vezes");
    assert_eq!(calls_distinct, 3);
    assert_eq!(bytes, bytes_distinct, "o JPEG mudou");
}
