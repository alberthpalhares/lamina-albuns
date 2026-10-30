//! Fotos entre lâminas pela IA e na tela (spec ML, regras 13 e 14): `trocar_fotos` com
//! `lamina_b`, `mover_foto`, a mesma sequência pela janela e pela IA gravando o mesmo arquivo, e o
//! quadro de origem vazio na tela e não no papel (ML-B12 a ML-B14).

use lamina_core::{FrameContent, Project};
use serde_json::json;

use super::common::Base;
use super::ia_common::{Lab, album_project};
use crate::agent::ToolError;
use crate::editor::CrossTarget;
use crate::edits::spread_photos;
use crate::motion::Tier;
use crate::{Editor, ImageSize, RollSource, export_project, render_spread_image};

fn photo_at(p: &Project, spread: usize, frame: usize) -> Option<u64> {
    match &p.spreads[spread].frames[frame].content {
        FrameContent::Photo { photo, .. } => Some(photo.get()),
        _ => None,
    }
}

#[test]
fn ml_b12_ia_troca_fotos_entre_laminas() {
    let base = Base::with_photos(30, (600, 400));
    let (mut lab, _) = Lab::opened(&base, false);
    let (a, b) = (photo_at(lab.project(), 2, 0), photo_at(lab.project(), 4, 1));
    let reply = lab.ok(
        "trocar_fotos",
        json!({ "lamina": 3, "quadro_a": 1, "lamina_b": 5, "quadro_b": 2 }),
    );
    assert_eq!(
        (photo_at(lab.project(), 2, 0), photo_at(lab.project(), 4, 1)),
        (b, a)
    );
    assert_eq!(reply.data["laminas"].as_array().map(Vec::len), Some(2));
    assert_eq!(
        lab.labels().last().map(String::as_str),
        Some("IA: Trocar fotos · lâminas 3 e 5")
    );
    // Sem `lamina_b`, como sempre (IA-B6).
    lab.ok(
        "trocar_fotos",
        json!({ "lamina": 2, "quadro_a": 1, "quadro_b": 2 }),
    );
    assert_eq!(
        lab.labels().last().map(String::as_str),
        Some("IA: Trocar fotos · lâmina 2")
    );
}

#[test]
fn ml_b13_ia_move_uma_foto_e_grava_o_mesmo_arquivo_que_a_janela() {
    let base = Base::with_photos(30, (600, 400));
    let (mut lab, file) = Lab::opened(&base, false);
    // Esvazia o quadro 2 da lâmina 5, para o mover encher (tirar_foto agora leva o quadro junto,
    // spec FQ, regra 3).
    let cleared = lab
        .editor
        .session_mut()
        .unwrap()
        .edit("Esvaziar", |p| crate::edits::clear_frame(p, 4, 1));
    cleared.unwrap();
    let a = photo_at(lab.project(), 2, 1);
    lab.ok(
        "mover_foto",
        json!({ "lamina": 3, "quadro": 2, "para_lamina": 5, "para_quadro": 2 }),
    );
    assert_eq!(photo_at(lab.project(), 4, 1), a);
    assert_eq!(photo_at(lab.project(), 2, 1), None);
    lab.ok(
        "mover_foto",
        json!({ "lamina": 3, "quadro": 1, "para_lamina": 5 }),
    );
    let labels = lab.labels();
    assert_eq!(
        labels[labels.len() - 2..],
        [
            "IA: Mover foto · lâmina 3 → 5",
            "IA: Mover foto · lâmina 3 → 5"
        ]
    );
    let before = lab.project().clone();
    let empty = lab
        .call(
            "mover_foto",
            json!({ "lamina": 3, "quadro": 2, "para_lamina": 5 }),
        )
        .unwrap_err();
    assert!(matches!(
        empty,
        ToolError::NothingToMove {
            spread: 3,
            frame: 2
        }
    ));
    assert_eq!(
        empty.to_string(),
        "O quadro 2 da lâmina 3 está vazio; não há foto para mover."
    );
    // Um texto na lâmina 3 não sai do lugar.
    lab.ok(
        "adicionar_texto",
        json!({ "lamina": 3, "pagina": "esquerda", "texto": "Festa" }),
    );
    let text = lab.project().spreads[2].frames.len();
    let err = lab
        .call(
            "mover_foto",
            json!({ "lamina": 3, "quadro": text, "para_lamina": 5 }),
        )
        .unwrap_err();
    assert!(matches!(err, ToolError::TextFrame { spread: 3, .. }));
    lab.ok("desfazer", json!({}));
    assert_eq!(
        lab.project().spreads,
        before.spreads,
        "a recusa não mudou nada"
    );
    lab.ok("salvar_album", json!({}));
    let by_ia = std::fs::read(&file).unwrap();

    // A mesma sequência pela janela (o editor).
    let mut editor = Editor::new();
    editor.adopt(album_project(&base));
    let cleared = editor
        .session_mut()
        .unwrap()
        .edit("Esvaziar", |p| crate::edits::clear_frame(p, 4, 1));
    cleared.unwrap();
    assert!(editor.go_to_spread(2));
    assert!(editor.move_photo(
        1,
        CrossTarget::Frame {
            spread: 4,
            frame: 1
        }
    ));
    assert!(editor.move_photo(0, CrossTarget::Spread(4)));
    let saved = base.path("janela.lamina");
    editor.session_mut().unwrap().save_as(&saved).unwrap();
    let by_window = std::fs::read(&saved).unwrap();
    assert!(by_ia == by_window, "o .lamina da IA e o da janela diferem");
}

#[test]
fn ml_b14_origem_vazia_na_tela_e_nao_no_papel() {
    let base = Base::with_photos(30, (600, 400));
    let mut editor = Editor::new();
    editor.adopt(album_project(&base));
    editor.go_to_spread(2);
    assert!(editor.move_photo(1, CrossTarget::Spread(4)));
    let moved = editor.session().unwrap().project().clone();
    assert_eq!(moved.spreads[2].frames[1].content, FrameContent::Empty);
    // Na vista, o quadro vazio (o X é da janela); na pré-impressão, o aviso de quadro vazio.
    let view = crate::spread_view(
        &moved,
        2,
        crate::Area {
            width: 1.0,
            height: 1.0,
            margin: 0.0,
        },
        &[],
    )
    .unwrap();
    assert!(matches!(view.frames[1].kind, crate::FrameKind::Empty));
    let warnings = editor.session().unwrap().warnings_for(2);
    assert!(
        warnings.iter().any(|w| w.to_string().contains("vazio")),
        "{warnings:?}"
    );
    // Na imagem do rolo, o X (o meio do quadro mais claro que o cinza).
    let source = RollSource::new(&moved, 2, &base.cache(), Tier::Small);
    let size = ImageSize {
        width: 384,
        shown: 384.0,
    };
    render_spread_image(&moved, (2, size), &source, &[]).unwrap();
    // A exportação é igual à de um álbum em que o mesmo quadro foi esvaziado com Delete.
    let mut deleted = album_project(&base);
    let a = spread_photos(&deleted, 2)[1];
    deleted.spreads[2].frames[1].content = FrameContent::Empty;
    let mut five = spread_photos(&deleted, 4);
    five.push(a);
    crate::edits::layout_with(&mut deleted, 4, &five).unwrap();
    let options = lamina_export::ExportOptions::default();
    let one = export_project(&moved, &base.path("a"), &options, &|_, _| {}).unwrap();
    let two = export_project(&deleted, &base.path("b"), &options, &|_, _| {}).unwrap();
    let read = |r: &lamina_export::ExportReport| {
        r.files
            .iter()
            .map(|f| std::fs::read(f).unwrap())
            .collect::<Vec<_>>()
    };
    assert_eq!(read(&one), read(&two));
}
