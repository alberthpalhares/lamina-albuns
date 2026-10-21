//! OP-B7 (spec `opacidade-da-foto`): fotos a 30%, 60% e 100%, com borda e cantos, saem com os
//! mesmos bytes em JPEG, PDF e TIFF, exportadas por dois processos diferentes.

#![allow(clippy::unwrap_used)]

mod common;

use common::{Base, text};
use lamina_core::{Border, FrameContent, Mm, Opacity, Project, Rgb};

#[test]
fn op_b7_mesmos_arquivos_por_dois_processos() {
    let base = Base::with_photos(6, (300, 200));
    let out = base.build("saida", &["--laminas", "2", "--nome", "album"]);
    assert!(out.status.success(), "{}", text(&out.stderr));
    let path = base.path("saida/album.lamina");
    let mut project = Project::from_json(&std::fs::read_to_string(&path).unwrap()).unwrap();
    let percents = [30, 60, 100];
    let photos = project
        .spreads
        .iter_mut()
        .flat_map(|s| s.frames.iter_mut())
        .filter(|f| matches!(f.content, FrameContent::Photo { .. }));
    for (k, frame) in photos.enumerate() {
        frame.opacity = Opacity::new(percents[k % 3]).unwrap();
        frame.border = Border {
            width: Mm(0.5),
            color: Rgb::WHITE,
        };
        frame.radius = Mm(2.0);
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
