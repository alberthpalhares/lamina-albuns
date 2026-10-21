//! O clique duplo no seletor volta ao neutro (spec AF, regra 14): uma edição própria por controle,
//! nada quando já está no neutro, e o endireitar pedindo um quadro só (AF-B24 e AF-B25).

use super::af::{adjust_of, labels, photo_frames, selected};
use super::tl_common::{editor_at, three_spreads};
use crate::Command;
use crate::editor::{AdjustChange, AdjustControl};

#[test]
fn af_b24_clique_duplo_volta_ao_neutro() {
    let (mut editor, _, photo) = selected();
    for change in [AdjustChange::Angle(30), AdjustChange::Contrast(-20)] {
        assert!(editor.adjust(change));
        editor.end_adjust();
    }
    // O brilho recém-arrastado, sem soltar: o clique duplo não pode emendar com ele.
    assert!(editor.adjust(AdjustChange::Brightness(10)));
    assert!(editor.adjust(AdjustChange::Brightness(30)));
    let before = labels(&editor).len();
    assert!(editor.reset_adjust(AdjustControl::Brightness));
    assert_eq!(adjust_of(&editor, photo).brightness, 0);
    assert_eq!(labels(&editor).len(), before + 1, "uma edição nova");
    assert_eq!(labels(&editor).last().unwrap(), "Brilho");
    assert!(editor.reset_adjust(AdjustControl::Angle));
    assert_eq!(adjust_of(&editor, photo).angle_tenths, 0);
    assert_eq!(labels(&editor).last().unwrap(), "Endireitar foto");
    assert_eq!(
        adjust_of(&editor, photo).contrast,
        -20,
        "só o controle clicado"
    );
    assert!(editor.run(Command::Undo));
    assert_eq!(adjust_of(&editor, photo).angle_tenths, 30);
    assert!(editor.run(Command::Undo));
    assert_eq!(adjust_of(&editor, photo).brightness, 30);
}

#[test]
fn af_b25_neutro_sem_efeito_e_varios_quadros() {
    let (mut editor, _, _) = selected();
    let before = labels(&editor).len();
    assert!(
        !editor.reset_adjust(AdjustControl::Contrast),
        "já neutro: nada muda"
    );
    assert_eq!(labels(&editor).len(), before, "sem edição");

    let mut editor = editor_at(three_spreads(), 1);
    let frames: Vec<_> = photo_frames(&editor).into_iter().take(3).collect();
    assert_eq!(frames.len(), 3);
    for ((f, _), v) in frames.iter().zip([10, 20, 0]) {
        editor.select_frame(Some(*f));
        editor.adjust(AdjustChange::Brightness(v));
        editor.end_adjust();
    }
    assert!(editor.run(Command::Organize));
    for (k, (f, _)) in frames.iter().enumerate() {
        editor.organize_click(*f, k > 0, false);
    }
    let before = labels(&editor).len();
    assert!(editor.reset_adjust(AdjustControl::Brightness));
    assert_eq!(labels(&editor).len(), before + 1, "uma edição para as três");
    for (_, p) in &frames {
        assert_eq!(adjust_of(&editor, *p).brightness, 0);
    }
    assert!(
        !editor.reset_adjust(AdjustControl::Angle),
        "o ângulo pede um quadro só"
    );
    assert_eq!(labels(&editor).len(), before + 1);
}
