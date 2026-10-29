//! Cenários BR de importar, dos gestos de uma lâmina no álbum em branco, da aparência nas lâminas
//! feitas à mão e da IA (BR-B9 a BR-B15).

use lamina_core::{FrameContent, PhotoId, Project};
use lamina_layout::Reuse;
use serde_json::json;

use super::br::{blank, photos_by_spread, style};
use super::common::Base;
use super::ia_common::{Lab, read};
use super::tl_common::{H, built, editor_at, spec};
use super::tr::report;
use crate::agent::{SPREADS_NEED_BUILD, ToolError};
use crate::building::{ALL_USED, ONE_SPREAD_LIMIT};
use crate::new_project::put_style;
use crate::{Command, DropPlace, Editor, Part, Session, create_album};

fn last_label(editor: &Editor) -> String {
    let history = editor.session().unwrap().history_list();
    history.steps.last().unwrap().label.clone()
}

/// Um álbum em branco (sem arquivo de fotos) com `count` fotos e a aparência de [`style`].
fn blank_editor(count: usize) -> (Editor, Vec<PhotoId>) {
    let mut project = Project::new("branco", spec(30.0, 30.0));
    put_style(&mut project, &style());
    let ids = (0..count)
        .map(|i| project.add_photo(format!("f{i:02}.jpg"), H.0, H.1))
        .collect();
    let mut editor = Editor::new();
    editor.adopt(project);
    (editor, ids)
}

#[test]
fn br_b9_importar_nao_monta() {
    let base = Base::with_photos(40, (600, 400));
    let mut first = Editor::new();
    first.adopt(blank(&base));
    let mut six = editor_at(built(spec(30.0, 30.0), &[H; 24], 6), 0);
    for editor in [&mut first, &mut six] {
        let before = editor.session().unwrap().project().spreads.len();
        editor.add_photos(&report(30)).unwrap();
        assert_eq!(editor.session().unwrap().project().spreads.len(), before);
        let steps = editor.session().unwrap().history_list().steps;
        assert_eq!(steps.len(), 1);
        assert_eq!(steps[0].label, "Importar 30 fotos · álbum");
    }
    assert_eq!(first.session().unwrap().project().photos.len(), 70);
}

#[test]
fn br_b10_a_primeira_lamina_a_mao() {
    let (mut editor, ids) = blank_editor(40);
    let five = ids[..5].to_vec();
    editor.set_selection(five.clone());
    let panel = editor.panel(Part::Spread);
    assert!(panel.unavailable.is_none() && !panel.options.is_empty());
    let mut made = Vec::new();
    for (k, label) in [
        (0, "Nova lâmina com 5 fotos · lâmina 1"),
        (1, "Nova lâmina com 5 fotos · lâmina 1"),
        (2, "Nova lâmina com a seleção · lâmina 1"),
    ] {
        editor.set_selection(five.clone());
        let done = match k {
            0 => editor.drop_photos(DropPlace::Spread(0), &five),
            1 => editor.run(Command::BuildFromSelection),
            _ => editor.run(Command::NewSpreadFromSelection),
        };
        assert!(done);
        assert_eq!(last_label(&editor), label);
        let project = editor.session().unwrap().project();
        assert_eq!(project.spreads.len(), 1);
        made.push(
            project.spreads[0]
                .frames
                .iter()
                .map(|f| f.rect)
                .collect::<Vec<_>>(),
        );
        assert!(editor.run(Command::Undo));
    }
    assert!(
        made.iter().all(|m| *m == made[0]),
        "montada como Montar lâmina"
    );
}

#[test]
fn br_b11_mais_do_que_uma_lamina_leva() {
    let (mut editor, ids) = blank_editor(40);
    let thirty = ids[..30].to_vec();
    editor.set_selection(thirty.clone());
    assert!(!editor.drop_photos(DropPlace::Spread(0), &thirty));
    assert_eq!(editor.take_notice().as_deref(), Some(ONE_SPREAD_LIMIT));
    assert!(editor.available(Command::BuildFromSelection));
    assert!(!editor.run(Command::BuildFromSelection));
    assert_eq!(editor.take_notice().as_deref(), Some(ONE_SPREAD_LIMIT));
    assert!(editor.session().unwrap().project().spreads.is_empty());
}

#[test]
fn br_b12_a_aparencia_do_album_nas_laminas_feitas_a_mao() {
    let (mut editor, ids) = blank_editor(20);
    assert!(editor.drop_photos(DropPlace::Spread(0), &ids[..4]));
    editor.set_selection(ids[4..7].to_vec());
    assert!(editor.run(Command::NewSpreadFromSelection));
    assert!(editor.run(Command::NewSpread));
    assert!(editor.drop_photos(DropPlace::Spread(2), &ids[7..8]));
    editor.build_album(None, Reuse::Medium).unwrap();
    let project = editor.session().unwrap().project();
    assert!(project.spreads.len() >= 5);
    let look = style().look;
    for spread in &project.spreads {
        assert_eq!(spread.background, look.background);
        assert_eq!(project.spread_gap(spread), lamina_core::Mm(4.0));
        for frame in &spread.frames {
            assert!(matches!(frame.content, FrameContent::Photo { .. }));
            assert_eq!((frame.border, frame.radius), (look.border, look.radius));
        }
    }
}

#[test]
fn br_b13_o_mais_lamina_nova_continua_montando_aquela_lamina() {
    let mut editor = editor_at(built(spec(30.0, 30.0), &[H; 24], 6), 0);
    let before = editor.session().unwrap().project().spreads.clone();
    editor.add_photos_as_spread(&report(4), 6).unwrap();
    let project = editor.session().unwrap().project();
    assert_eq!(project.spreads.len(), 7);
    assert_eq!(project.spreads[6].frames.len(), 4);
    let panel: Vec<PhotoId> = project.photos[..4].iter().map(|p| p.id).collect();
    assert!(editor.drop_photos(DropPlace::NewSpread(7), &panel));
    let project = editor.session().unwrap().project();
    assert_eq!(project.spreads.len(), 8);
    assert_eq!(project.spreads[..6], before[..]);
    assert_eq!(editor.session().unwrap().history_list().steps.len(), 2);
}

#[test]
fn br_b14_so_laminas_vazias() {
    let (mut editor, _) = blank_editor(12);
    assert!(editor.run(Command::NewSpread));
    assert_eq!(editor.session().unwrap().project().spreads.len(), 1);
    let steps = editor.session().unwrap().history_list().steps.len();
    editor.build_album(Some(3), Reuse::Medium).unwrap();
    let project = editor.session().unwrap().project();
    assert_eq!(project.spreads.len(), 3);
    assert!(photos_by_spread(project).iter().all(|p| !p.is_empty()));
    let history = editor.session().unwrap().history_list().steps;
    assert_eq!(history.len(), steps + 1, "numa edição só");
    assert_eq!(last_label(&editor), "Montar 3 lâminas com 12 fotos · álbum");
}

#[test]
fn br_b15_a_ia_comeca_em_branco_e_monta_quando_pede() {
    let base = Base::with_photos(40, (600, 400));
    let roots = vec![base.path("fotos"), base.path("ia")];
    let mut lab = Lab::new(&base, roots, true);
    let target = base.path("ia").join("album.lamina");
    let quadrado = "Quadrado 30 × 30";
    let args =
        json!({ "pasta_fotos": base.path("fotos"), "salvar_em": target, "preset": quadrado });
    lab.ok("novo_album", args);
    // Igual ao Novo álbum da janela com as mesmas escolhas e a caixa desmarcada.
    let preset = crate::builtin_presets()
        .into_iter()
        .find(|p| p.name == quadrado)
        .unwrap();
    let folder = base.path("fotos");
    let window = crate::NewAlbum {
        // O nome que a janela recebe: o do arquivo pedido (relato do teste pelo MCP, IA-B26).
        name: Some("album".into()),
        folder,
        spec: preset.fields.to_spec().unwrap(),
        limits: preset.limits,
        export: preset.export,
        build: false,
        ..base.new_album(None)
    };
    let (by_window, _) = create_album(&window, &base.cache(), &|_| {}).unwrap();
    let mut copy = Editor::new();
    copy.adopt(by_window.project.clone());
    std::fs::create_dir_all(base.path("janela")).unwrap();
    let saved = Session::new_unsaved(by_window.project)
        .save_as(&base.path("janela").join("album.lamina"))
        .unwrap();
    assert!(read(&target) == read(&saved), "os dois .lamina diferem");
    assert!(lab.project().spreads.is_empty());

    lab.ok("montar_album", json!({ "laminas": 10 }));
    copy.build_album(Some(10), Reuse::Medium).unwrap();
    assert_eq!(
        lab.project().spreads,
        copy.session().unwrap().project().spreads
    );
    assert_eq!(
        lab.labels().last().map(String::as_str),
        Some("IA: Montar 10 lâminas com 40 fotos · álbum")
    );
    let again = lab.call("montar_album", json!({}));
    assert!(matches!(&again, Err(ToolError::Refused(m)) if m == ALL_USED));

    let other = base.path("ia").join("outro.lamina");
    let refused = lab.call(
        "novo_album",
        json!({ "pasta_fotos": base.path("fotos"), "salvar_em": other, "laminas": 8 }),
    );
    assert!(matches!(&refused, Err(ToolError::Refused(m)) if m == SPREADS_NEED_BUILD));
    assert!(!other.exists());
    let dialog = lab.call(
        "executar_comando",
        json!({ "comando": "lamina.montar-album" }),
    );
    assert!(matches!(
        dialog,
        Err(ToolError::DialogCommand {
            tool: "montar_album",
            ..
        })
    ));
}
