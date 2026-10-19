//! Cenários da spec IA do IA-M2 que mudam a cara de uma lâmina: IA-B20 (recortar) e IA-B21
//! (aparência e texto), com as recusas.

use lamina_core::{Crop, FrameContent, Mm, PageSide, Project, Rgb};
use serde_json::json;

use super::common::Base;
use super::ia_common::Lab;
use crate::agent::ToolError;
use crate::editor::{crop_for, set_crop};
use crate::frame_form::font_names;
use crate::{Editor, EditorError, TextChange};

fn crop_of(project: &Project, index: usize, frame: usize) -> Crop {
    match &project.spreads[index].frames[frame].content {
        FrameContent::Photo { crop, .. } => *crop,
        other => panic!("o quadro devia ter foto: {other:?}"),
    }
}

/// O primeiro quadro da lâmina `index` com uma foto horizontal.
fn landscape_frame(project: &Project, index: usize) -> usize {
    project.spreads[index]
        .frames
        .iter()
        .position(|f| match &f.content {
            FrameContent::Photo { photo, .. } => {
                project.photo(*photo).unwrap().aspect_ratio() > 1.0
            }
            _ => false,
        })
        .expect("a lâmina devia ter uma foto horizontal")
}

#[test]
fn ia_b20_recortar() {
    let base = Base::with_photos(30, (600, 400));
    let (mut lab, file) = Lab::opened(&base, true);
    let frame = landscape_frame(lab.project(), 1);
    let quadro = frame + 1;
    // Na janela: modo recorte, zoom 1,5 e depois a foto arrastada até a borda direita.
    let mut window = Editor::new();
    window.open(&file).unwrap();
    window.go_to_spread(1);
    assert!(window.enter_crop(frame));
    window.crop_zoom(1.5);
    window.exit_crop();
    let zoomed = crop_of(window.session().unwrap().project(), 1, frame);
    lab.ok(
        "recortar",
        json!({ "lamina": 2, "quadro": quadro, "zoom": 1.5 }),
    );
    assert_eq!(crop_of(lab.project(), 1, frame), zoomed);

    // O centro (0,3; 0,5) da spec: preso à foto como no modo recorte.
    let wanted = Crop {
        zoom: 1.5,
        center_x: 0.3,
        center_y: 0.5,
    };
    let expected = crop_for(lab.project(), 1, frame, wanted).unwrap();
    let reply = lab.ok(
        "recortar",
        json!({ "lamina": 2, "quadro": quadro, "zoom": 1.5, "centro_x": 0.3, "centro_y": 0.5 }),
    );
    assert_eq!(crop_of(lab.project(), 1, frame), expected);
    assert_eq!(
        reply.data["lamina"]["quadros"][frame]["recorte"]["zoom"],
        1.5
    );

    window.enter_crop(frame);
    window.crop_drag(-5.0, 0.0);
    window.exit_crop();
    let edge = crop_of(window.session().unwrap().project(), 1, frame);
    lab.ok(
        "recortar",
        json!({ "lamina": 2, "quadro": quadro, "centro_x": 1.0 }),
    );
    assert_eq!(crop_of(lab.project(), 1, frame).center_x, edge.center_x);
    assert_eq!(lab.labels(), ["IA: Recortar foto · lâmina 2"; 3]);

    // O mesmo recorte de novo não vira passo.
    let same = lab.ok(
        "recortar",
        json!({ "lamina": 2, "quadro": quadro, "centro_x": 1.0 }),
    );
    assert_eq!(same.text, "O recorte já estava assim; nada mudou.");
    assert_eq!(lab.labels().len(), 3);

    let before = lab.project().clone();
    let err = lab
        .call(
            "recortar",
            json!({ "lamina": 2, "quadro": quadro, "zoom": 0.5 }),
        )
        .unwrap_err();
    assert!(matches!(err, ToolError::BadCrop));
    assert_eq!(
        err.to_string(),
        "O zoom vai de 1 (a foto cobre o quadro inteiro) a 8, e o centro vai de 0 a 1 (0,5 é o \
         meio da foto)."
    );
    assert_eq!(lab.project(), &before);
    assert_eq!(lab.labels().len(), 3);
}

#[test]
fn definir_recorte_grava_e_recusa_quadro_sem_foto() {
    let base = Base::with_photos(30, (600, 400));
    let mut project = super::ia_common::album_project(&base);
    let frame = landscape_frame(&project, 1);
    let wanted = Crop {
        zoom: 2.0,
        center_x: 0.5,
        center_y: 0.5,
    };
    let expected = crop_for(&project, 1, frame, wanted).unwrap();
    assert!((expected.zoom - 2.0).abs() < 1e-12);
    let set = set_crop(&mut project, 1, frame, wanted).unwrap();
    assert_eq!(set, expected);
    assert_eq!(crop_of(&project, 1, frame), expected);
    project.spreads[1].frames[frame].content = FrameContent::Empty;
    assert!(matches!(
        set_crop(&mut project, 1, frame, wanted),
        Err(EditorError::NoFrame)
    ));
    let bad = Crop {
        zoom: 0.5,
        ..wanted
    };
    project.spreads[1].frames[frame].content = FrameContent::Photo {
        photo: project.photos[0].id,
        crop: Crop::default(),
    };
    assert!(matches!(
        crop_for(&project, 1, frame, bad),
        Err(EditorError::Core(lamina_core::CoreError::InvalidCrop(_)))
    ));
}

#[test]
fn ia_b21_aparencia_e_texto() {
    let base = Base::with_photos(30, (600, 400));
    let (mut lab, file) = Lab::opened(&base, true);
    let font = font_names()[1];
    lab.ok(
        "aparencia",
        json!({ "lamina": 2, "espaco_mm": 5.0, "borda_mm": 1.0 }),
    );
    let added = lab.ok(
        "adicionar_texto",
        json!({ "lamina": 2, "pagina": "esquerda", "texto": "Cerimônia" }),
    );
    let quadro = added.data["quadro"].as_u64().unwrap();
    lab.ok(
        "editar_texto",
        json!({ "lamina": 2, "quadro": quadro, "fonte": font }),
    );
    assert_eq!(
        lab.labels(),
        [
            "IA: Aparência · lâmina 2",
            "IA: Novo texto · lâmina 2",
            "IA: Editar texto · lâmina 2"
        ]
    );
    // As abas Aparência e Quadro com os mesmos valores.
    let mut window = Editor::new();
    window.open(&file).unwrap();
    window.go_to_spread(1);
    assert_eq!(window.set_gap(Mm(5.0)), None);
    assert!(window.set_border(Mm(1.0), Rgb::WHITE));
    assert!(window.new_text(Some(PageSide::Left)));
    assert!(window.change_text(TextChange::Text("Cerimônia".into())));
    assert!(window.change_text(TextChange::Font(font.into())));
    assert_eq!(
        lab.project().spreads[1],
        window.session().unwrap().project().spreads[1]
    );
}

#[test]
fn aparencia_e_texto_recusam_sem_mudar_nada() {
    let base = Base::with_photos(30, (600, 400));
    let (mut lab, _) = Lab::opened(&base, true);
    let before = lab.project().clone();
    let refusals = [
        ("aparencia", json!({ "lamina": 2, "espaco_mm": 25.0 })),
        ("aparencia", json!({ "lamina": 2, "fundo": "roxo-sereno" })),
        ("aparencia", json!({ "lamina": 2 })),
        (
            "editar_texto",
            json!({ "lamina": 2, "quadro": 1, "texto": "x" }),
        ),
        (
            "adicionar_texto",
            json!({ "lamina": 2, "pagina": "direita", "texto": "x", "fonte": "Comic" }),
        ),
        (
            "adicionar_texto",
            json!({ "lamina": 2, "pagina": "direita", "texto": "x", "tamanho": 500.0 }),
        ),
    ];
    let errors: Vec<ToolError> = refusals
        .into_iter()
        .map(|(name, args)| lab.call(name, args).unwrap_err())
        .collect();
    assert!(matches!(
        errors[0],
        ToolError::OutOfRange("O espaço vai de 0 a 20 mm.")
    ));
    assert!(errors[1].to_string().starts_with(
        "A cor roxo-sereno não existe. Use uma cor como #F5F0E6 ou um nome da paleta: Branco, Marfim"
    ));
    assert!(matches!(errors[2], ToolError::NothingAsked(_)));
    assert!(matches!(
        errors[3],
        ToolError::NotText {
            spread: 2,
            frame: 1
        }
    ));
    assert!(matches!(&errors[4], ToolError::NoFont { name, .. } if name == "Comic"));
    assert!(matches!(
        errors[5],
        ToolError::OutOfRange("O tamanho vai de 4 a 300 pt.")
    ));
    assert_eq!(lab.project(), &before);
    assert!(lab.labels().is_empty());
}

#[test]
fn aparencia_com_todas_e_cor_pelo_nome() {
    let base = Base::with_photos(30, (600, 400));
    let (mut lab, _) = Lab::opened(&base, true);
    let reply = lab.ok(
        "aparencia",
        json!({ "lamina": 3, "fundo": "marfim", "todas": true }),
    );
    assert_eq!(reply.data["lamina"]["fundo"], "#FAF6EE");
    assert!(lab.project().spreads.iter().all(|s| s.background
        == Rgb {
            r: 250,
            g: 246,
            b: 238
        }));
    assert_eq!(lab.labels(), ["IA: Aparência · todas as lâminas"]);
}
