//! AF-B14 (spec `ajustes-da-foto`): fotos giradas e em preto e branco saem com os mesmos bytes
//! em JPEG, PDF e TIFF, exportadas por dois processos diferentes.

#![allow(clippy::unwrap_used)]

mod common;

use common::{Base, text};
use lamina_core::{PhotoAdjust, Project};

#[test]
fn af_b14_mesmos_arquivos_por_dois_processos() {
    let base = Base::with_photos(6, (300, 200));
    let out = base.build("saida", &["--laminas", "2", "--nome", "album"]);
    assert!(out.status.success(), "{}", text(&out.stderr));
    // Ajustes em todas as fotos: giros dos dois lados, preto e branco, brilho e contraste.
    let path = base.path("saida/album.lamina");
    let mut project = Project::from_json(&std::fs::read_to_string(&path).unwrap()).unwrap();
    for (k, photo) in project.photos.iter_mut().enumerate() {
        let k = i32::try_from(k).unwrap();
        photo.adjust = PhotoAdjust {
            angle_tenths: if k % 2 == 0 { 3 + k * 40 } else { -7 - k * 30 },
            brightness: 10 * k - 20,
            contrast: 15 - 5 * k,
            bw: k % 3 == 0,
            ..PhotoAdjust::default()
        };
    }
    std::fs::write(&path, project.to_json().unwrap()).unwrap();
    for formato in ["jpeg", "pdf", "tiff"] {
        let mut outputs = Vec::new();
        for run in ["a", "b"] {
            let folder = format!("{formato}-{run}");
            let out = base.lamina(&[
                "exportar",
                "saida/album.lamina",
                "--saida",
                &folder,
                "--formato",
                formato,
            ]);
            assert!(out.status.success(), "{}", text(&out.stderr));
            let mut names: Vec<_> = std::fs::read_dir(base.path(&folder))
                .unwrap()
                .map(|e| e.unwrap().file_name().into_string().unwrap())
                .collect();
            names.sort();
            let bytes: Vec<Vec<u8>> = names
                .iter()
                .map(|n| std::fs::read(base.path(&folder).join(n)).unwrap())
                .collect();
            outputs.push((names, bytes));
        }
        assert!(!outputs[0].0.is_empty(), "{formato}: nada exportado");
        assert!(
            outputs[0] == outputs[1],
            "{formato}: os dois processos diferem"
        );
    }
}
