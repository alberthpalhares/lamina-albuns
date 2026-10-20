//! Spec `novo-projeto` (NP), regras 1, 3 e 11: "Montagem do álbum…" num álbum aberto, e a unidade
//! e o último formato lembrados com a janela.

use lamina_core::{CropLevel, DesignMode, Mounting};

use super::tl_common::{editor_at, three_spreads};
use crate::new_project::{Format, Unit};
use crate::{Command, Menu, StripsStore};

#[test]
fn montagem_do_album_vale_para_as_proximas_sugestoes_e_se_desfaz() {
    let mut editor = editor_at(three_spreads(), 1);
    assert!(editor.available(Command::MountAlbum));
    assert_eq!(Command::MountAlbum.name(), "arquivo.montagem");
    assert_eq!(Command::MountAlbum.label(), "Montagem do álbum…");
    assert_eq!(Command::MountAlbum.menu(), Menu::File);
    let before = editor.session().unwrap().project().spreads.clone();
    let whole = Mounting {
        crop: CropLevel::Whole,
        ..Mounting::default()
    };
    editor.set_mounting(whole, false).unwrap();
    let project = editor.session().unwrap().project();
    assert_eq!(project.mounting, whole);
    assert_eq!(project.spec.mode(), DesignMode::Page);
    assert_eq!(project.spreads, before, "sem refazer as lâminas");
    // As próximas sugestões já obedecem: nenhuma recorta.
    let panel = editor.panel(crate::Part::Spread);
    assert!(
        panel
            .options
            .iter()
            .all(|c| c.crop.iter().all(|&x| x < 1e-6)),
        "{:?}",
        panel.options.iter().map(|c| &c.family).collect::<Vec<_>>()
    );
    assert!(editor.run(Command::Undo));
    let back = editor.session().unwrap().project();
    assert_eq!(back.mounting, Mounting::default());
    assert_eq!(back.spec.mode(), DesignMode::Spread);
    assert!(!crate::Editor::new().available(Command::MountAlbum));
}

#[test]
fn unidade_e_ultimo_formato_ficam_com_a_janela() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("janela.json");
    // Um `janela.json` de antes não tinha esses campos: valem os padrões.
    std::fs::write(
        &file,
        r#"{ "laminas": true, "linha_do_tempo": false, "fotos": true }"#,
    )
    .unwrap();
    let mut editor = editor_at(three_spreads(), 0);
    editor.set_strips(StripsStore::load(&file));
    assert_eq!(editor.strips().unit, Unit::Cm);
    assert_eq!(editor.strips().format, Format::Square);
    assert!(!editor.strips().timeline);
    editor.remember_unit(Unit::Inch);
    editor.remember_format(Format::Landscape);
    let again = StripsStore::load(&file);
    assert_eq!(again.warning, None);
    assert_eq!(again.strips.unit, Unit::Inch);
    assert_eq!(again.strips.format, Format::Landscape);
    assert!(!again.strips.timeline, "as faixas continuam como estavam");
    let text = std::fs::read_to_string(&file).unwrap();
    assert!(text.contains("\"unidade\": \"pol\""), "{text}");
    assert!(
        text.contains("\"ultimo_formato\": \"horizontal\""),
        "{text}"
    );
}
