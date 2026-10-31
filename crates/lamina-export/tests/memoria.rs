//! LV-B18 (spec `leveza`): o pico de memória da exportação, medido pelo `dhat`. Fica num binário
//! de teste só dele, porque o `dhat` troca o alocador do programa.

#![allow(clippy::unwrap_used)]

use std::sync::Arc;

use image::RgbaImage;
use lamina_core::{AlbumSpec, Crop, Dpi, FrameContent, Mm, Photo, Project, RelRect};
use lamina_export::{ExportOptions, export};

#[global_allocator]
static ALLOC: dhat::Alloc = dhat::Alloc;

const MIB: u64 = 1024 * 1024;

/// Fotos de 24 MP: cada original aberto gasta uns 96 MB.
const SIZE: (u32, u32) = (6000, 4000);

/// 12 lâminas de 60 × 30 cm a 300 DPI, com 6 fotos diferentes em cada. O pico vem de quantas
/// lâminas e fotos ficam abertas ao mesmo tempo, então não muda com mais lâminas.
fn album() -> Project {
    let spec = AlbumSpec::new(Mm::from_cm(30.0), Mm::from_cm(30.0), Dpi::new(300).unwrap())
        .unwrap()
        .with_bleed(Mm(3.0))
        .unwrap();
    let mut project = Project::new("teste", spec);
    for s in 0..12 {
        let spread = project.add_spread();
        for n in 0..6u32 {
            let photo = project.add_photo(format!("f{s}_{n}.jpg"), SIZE.0, SIZE.1);
            let (col, row) = (f64::from(n % 2), f64::from(n / 2));
            let rect = RelRect::new(0.03 + col * 0.5, 0.04 + row * 0.32, 0.44, 0.29).unwrap();
            let content = FrameContent::Photo {
                photo,
                crop: Crop::default(),
            };
            project.add_frame(spread, rect, content).unwrap();
        }
    }
    project
}

#[test]
fn lv_b18_exportar_fotos_grandes_fica_abaixo_de_1_gb() {
    let _profiler = dhat::Profiler::builder().testing().build();
    let project = album();
    let dir = tempfile::tempdir().unwrap();
    let source = |_: &Photo, _: (u32, u32)| {
        let (w, h) = SIZE;
        // Um vetor preenchido de verdade: a memória de um original aberto.
        Ok(Arc::new(RgbaImage::from_pixel(
            w,
            h,
            image::Rgba([90, 120, 150, 255]),
        )))
    };
    let report = export(
        &project,
        dir.path(),
        &source,
        &ExportOptions::default(),
        &|_, _| {},
    )
    .unwrap();
    assert_eq!(report.files.len(), 12);
    let peak = dhat::HeapStats::get().max_bytes as u64 / MIB;
    println!("pico da exportação: {peak} MiB");
    assert!(peak < 1024, "pico de {peak} MiB");
}
