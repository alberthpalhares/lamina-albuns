//! Cenários da spec `controles-deslizantes` (CD): a escala e o zoom proporcionais ao logaritmo,
//! o giro até 180°, os botões de 90° e a volta ao neutro.

use lamina_core::{Crop, FrameContent, PhotoAdjust, PhotoId};

use super::tl_common::{editor_at, three_spreads};
use crate::editor::{AdjustChange, AdjustControl};
use crate::sliders::{log_position, log_value, wrap_turn};
use crate::{Command, Editor};

/// Na lâmina 1, com o primeiro quadro de foto selecionado: (editor, quadro, foto).
fn selected() -> (Editor, usize, PhotoId) {
    let mut editor = editor_at(three_spreads(), 0);
    let s = editor.session().unwrap();
    let (frame, photo) = s.project().spreads[0]
        .frames
        .iter()
        .enumerate()
        .find_map(|(i, f)| match &f.content {
            FrameContent::Photo { photo, .. } => Some((i, *photo)),
            _ => None,
        })
        .unwrap();
    editor.select_frame(Some(frame));
    (editor, frame, photo)
}

fn angle(editor: &Editor, photo: PhotoId) -> i32 {
    let s = editor.session().unwrap();
    s.project().photo(photo).unwrap().adjust.angle_tenths
}

fn steps(editor: &Editor) -> Vec<String> {
    let s = editor.session().unwrap();
    s.history_list()
        .steps
        .iter()
        .map(|x| x.label.clone())
        .collect()
}

#[test]
fn cd_b1_escala_proporcional_ao_logaritmo() {
    let near = |a: f64, b: f64| (a - b).abs() < 1e-9;
    assert!(near(log_value(0.0, 1.0, 8.0) * 100.0, 100.0));
    assert!(near(log_value(0.5, 1.0, 8.0), 8f64.sqrt()));
    assert!(near(log_value(1.0, 1.0, 8.0) * 100.0, 800.0));
    for pos in [0.0, 0.25, 0.5, 0.9, 1.0] {
        assert!(near(log_position(log_value(pos, 1.0, 8.0), 1.0, 8.0), pos));
    }
    // Cada décimo do trilho multiplica pelo mesmo fator.
    let tenth = log_value(0.1, 1.0, 8.0);
    assert!(near(
        log_value(0.6, 1.0, 8.0) / log_value(0.5, 1.0, 8.0),
        tenth
    ));
}

#[test]
fn cd_b2_um_arrasto_da_escala_e_um_passo() {
    let (mut editor, frame, _) = selected();
    let before = steps(&editor).len();
    for percent in [120.0, 150.0, 180.0] {
        let pos = log_position(percent / 100.0, 1.0, Crop::MAX_ZOOM);
        assert!(editor.set_scale_position(pos));
    }
    editor.end_adjust();
    let s = editor.session().unwrap();
    let zoom = match &s.project().spreads[0].frames[frame].content {
        FrameContent::Photo { crop, .. } => crop.zoom,
        _ => 0.0,
    };
    assert!((zoom - 1.8).abs() < 1e-9, "zoom {zoom}");
    let list = steps(&editor);
    assert_eq!(list.len(), before + 1);
    assert_eq!(list.last().unwrap(), "Escala · lâmina 1");
    assert!((editor.scale_position().unwrap() - log_position(1.8, 1.0, 8.0)).abs() < 1e-9);
}

#[test]
fn cd_b3_o_giro_vai_ate_180_graus() {
    let (mut editor, _, photo) = selected();
    assert!(editor.adjust(AdjustChange::Angle(1350)));
    assert_eq!(angle(&editor, photo), 1350);
    assert!(editor.adjust(AdjustChange::Angle(-1800)));
    assert_eq!(angle(&editor, photo), -1800);
    let out = PhotoAdjust {
        angle_tenths: 1810,
        ..PhotoAdjust::default()
    };
    assert!(!out.is_valid(), "181° fica fora da faixa");
}

#[test]
fn cd_b4_os_botoes_de_90_graus_dao_a_volta() {
    assert_eq!(wrap_turn(1700, 900), -1000);
    let (mut editor, _, photo) = selected();
    assert!(editor.adjust(AdjustChange::Angle(1700)));
    editor.end_adjust();
    let before = steps(&editor).len();
    assert!(editor.turn_quarter(1));
    assert_eq!(angle(&editor, photo), -1000);
    assert!(editor.turn_quarter(-1));
    assert_eq!(angle(&editor, photo), 1700);
    assert!(editor.turn_quarter(-1));
    assert_eq!(angle(&editor, photo), 800);
    let list = steps(&editor);
    assert_eq!(list.len(), before + 3, "um passo por clique");
    assert!(list[before..].iter().all(|l| l == "Girar 90° · lâmina 1"));
}

#[test]
fn cd_b5_girada_90_graus_a_foto_cobre_o_quadro() {
    let crop = Crop::default();
    for frame in [(1.0, 1.0), (3.0, 1.0)] {
        let r = crop.turned_region(6000.0, 4000.0, frame, 90.0);
        assert!(r.x >= -1e-9 && r.y >= -1e-9, "{frame:?}: {r:?}");
        assert!(r.x + r.width <= 6000.0 + 1e-6 && r.y + r.height <= 4000.0 + 1e-6);
        // A região tem a forma do quadro deitado: cobre o quadro inteiro depois do giro.
        let wanted = frame.1 / frame.0;
        assert!(
            (r.width / r.height - wanted).abs() < 1e-9,
            "{frame:?}: {r:?}"
        );
    }
}

#[test]
fn cd_b6_zoom_da_lamina_pelo_controle() {
    let mut editor = editor_at(three_spreads(), 0);
    assert!(editor.run(Command::Organize));
    let before = steps(&editor).len();
    assert_eq!(editor.view_zoom_position(), 0.0, "começa em Ajustar");
    assert!(editor.set_view_zoom_position(0.5).is_some());
    assert!((editor.view_zoom() - 8f64.sqrt()).abs() < 1e-9);
    assert!(editor.set_view_zoom_position(1.0).is_some());
    assert!((editor.view_zoom() - 8.0).abs() < 1e-9);
    assert!(
        editor.zoom_step(true, None, false).is_none(),
        "não passa de 8×"
    );
    assert_eq!(
        steps(&editor).len(),
        before,
        "o zoom não entra no histórico"
    );
    // Fora do modo organizar, o controle não existe.
    let mut outside = editor_at(three_spreads(), 0);
    assert!(outside.set_view_zoom_position(0.5).is_none());
}

#[test]
fn cd_b7_clique_duplo_volta_ao_neutro() {
    let (mut editor, _, photo) = selected();
    assert!(editor.set_photo_scale(250.0));
    assert!(editor.adjust(AdjustChange::Angle(300)));
    editor.end_adjust();
    let before = steps(&editor).len();
    assert!(editor.reset_scale());
    assert!(editor.reset_adjust(AdjustControl::Angle));
    assert_eq!(editor.photo_scale(), Some(100.0));
    assert_eq!(angle(&editor, photo), 0);
    assert_eq!(steps(&editor).len(), before + 2);
}
