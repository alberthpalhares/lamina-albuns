//! Achados da auditoria do Marco 2 (`docs/auditorias/2026-10-16-marco-2.md`) que viraram teste:
//! M2 (estado da tela ao trocar de projeto ou de lâmina) e M8 (trabalho em segundo plano).

use lamina_core::{Crop, FrameContent};

use super::common::Base;
use super::tl_common::{editor_at, three_spreads};
use crate::{Command, Editor, search_near};

/// Editor na lâmina 2, com o quadro 0 em modo recorte e um zoom da roda ainda pendente.
fn cropping() -> Editor {
    let mut editor = editor_at(three_spreads(), 1);
    editor.select_frame(Some(0));
    assert!(editor.run(Command::CropMode));
    assert!(editor.crop_zoom(2.0), "gesto pendente");
    editor
}

#[test]
fn abrir_outro_projeto_fecha_o_modo_recorte_sem_gravar_o_gesto() {
    let mut editor = cropping();
    editor.adopt(three_spreads());
    assert_eq!(editor.crop_mode(), None);
    assert_eq!(editor.selected_frame(), None);
    // O zoom pendente do projeto anterior não vira edição no novo.
    assert!(!editor.commit_crop());
    assert!(!editor.session().unwrap().can_undo());
}

#[test]
fn rc_b1_trocar_de_lamina_pela_faixa_sai_do_recorte_gravando_o_gesto() {
    let mut editor = cropping();
    assert!(editor.go_to_spread(2));
    assert_eq!(editor.crop_mode(), None);
    assert_eq!(editor.selected_frame(), None);
    let session = editor.session().unwrap();
    assert_eq!(session.current(), 2);
    // O gesto foi gravado na lâmina onde aconteceu.
    let FrameContent::Photo { crop, .. } = &session.project().spreads[1].frames[0].content else {
        panic!("o quadro tinha foto");
    };
    assert_ne!(*crop, Crop::default());
    assert!(!editor.go_to_spread(9));
}

#[test]
fn duplo_clique_na_foto_tambem_sai_do_recorte() {
    let mut editor = cropping();
    let project = editor.session().unwrap().project().clone();
    let FrameContent::Photo { photo, .. } = project.spreads[0].frames[0].content else {
        panic!("o quadro tinha foto");
    };
    assert!(editor.go_to_photo(photo));
    assert_eq!(editor.crop_mode(), None);
    assert_eq!(editor.session().unwrap().current(), 0);
}

#[test]
fn ocupado_so_deixa_o_sobre() {
    let mut editor = editor_at(three_spreads(), 0);
    editor.set_busy(true);
    for command in Command::ALL {
        assert_eq!(
            editor.available(command),
            command == Command::About,
            "{command:?}"
        );
    }
    assert!(!editor.run(Command::NextSpread));
    editor.set_busy(false);
    assert!(editor.available(Command::Open));
}

#[test]
fn procura_ao_abrir_nao_apaga_o_refazer() {
    let base = Base::with_photos(12, (300, 200));
    let file = base.save(base.project(3), "album.lamina");
    std::fs::create_dir_all(base.path("fotos/extras")).unwrap();
    std::fs::rename(
        base.path("fotos/f03.jpg"),
        base.path("fotos/extras/f03.jpg"),
    )
    .unwrap();
    let mut editor = Editor::new();
    editor.open(&file).unwrap();
    let found = {
        let s = editor.session().unwrap();
        search_near(s.project(), s.missing(), s.file(), &base.cache())
    };
    assert_eq!(found.len(), 1);
    // O fotógrafo editou e desfez antes de a procura terminar.
    assert!(editor.run(Command::NextLayout));
    assert!(editor.run(Command::Undo));
    let session = editor.session_mut().unwrap();
    assert_eq!(session.relink_after_open(&found), None);
    assert!(session.can_redo(), "o refazer continua");
    assert_eq!(session.missing().len(), 1);
    assert!(session.redo().is_some());
    assert_eq!(session.relink_after_open(&found), Some(1));
}

#[test]
fn trocar_o_layout_ou_o_espaco_mantem_borda_e_cantos() {
    use lamina_core::{Mm, Rgb};
    let mut editor = editor_at(three_spreads(), 1);
    assert!(editor.set_border(Mm(1.0), Rgb::WHITE));
    assert!(editor.set_radius(Mm(3.0)));
    let styled = |e: &Editor| {
        let s = e.session().unwrap();
        s.project().spreads[1]
            .frames
            .iter()
            .all(|f| f.border.width == Mm(1.0) && f.radius == Mm(3.0))
    };
    assert!(editor.run(Command::NextLayout));
    assert!(styled(&editor), "depois de trocar o layout");
    assert_eq!(editor.set_gap(Mm(6.0)), None);
    assert!(styled(&editor), "depois de mudar o espaço");
}
