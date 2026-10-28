//! Os ajustes da foto pelo editor (spec AF, regras 1, 2, 4 e 8, seção 6): o campo do ângulo, os
//! ajustes que seguem a foto, um gesto uma edição, as teclas no recorte, a grade, restaurar, a
//! escala e vários quadros (AF-B2, B9, B10, B11, B12, B17, B18 e B19).

use lamina_core::{FrameContent, PhotoAdjust, PhotoId};

use super::tl_common::{editor_at, three_spreads};
use crate::editor::{AdjustChange, BAD_ANGLE, BAD_SCALE, CropGrid};
use crate::keys::{Key, Shortcut};
use crate::{Command, Editor};

/// Na lâmina 1, com o primeiro quadro de foto selecionado.
pub(super) fn selected() -> (Editor, usize, PhotoId) {
    let mut editor = editor_at(three_spreads(), 1);
    let (frame, photo) = photo_frames(&editor)[0];
    editor.select_frame(Some(frame));
    (editor, frame, photo)
}

/// Os quadros de foto da lâmina atual: (índice, foto).
pub(super) fn photo_frames(editor: &Editor) -> Vec<(usize, PhotoId)> {
    let s = editor.session().unwrap();
    s.project().spreads[s.current()]
        .frames
        .iter()
        .enumerate()
        .filter_map(|(i, f)| match &f.content {
            FrameContent::Photo { photo, .. } => Some((i, *photo)),
            _ => None,
        })
        .collect()
}

pub(super) fn adjust_of(editor: &Editor, photo: PhotoId) -> PhotoAdjust {
    editor
        .session()
        .unwrap()
        .project()
        .photo(photo)
        .unwrap()
        .adjust
}

pub(super) fn labels(editor: &Editor) -> Vec<String> {
    let s = editor.session().unwrap();
    s.history_list()
        .steps
        .iter()
        .map(|st| st.label.split(" · ").next().unwrap_or_default().to_owned())
        .collect()
}

#[test]
fn af_b2_faixa_e_passo_do_angulo() {
    let (mut editor, _, photo) = selected();
    // Desde a spec CD, o limite é 180° (era 45°).
    assert_eq!(editor.set_angle_text("180,3"), Ok(true));
    assert_eq!(adjust_of(&editor, photo).angle_tenths, 1800);
    assert_eq!(editor.set_angle_text("0,25"), Ok(true));
    assert_eq!(adjust_of(&editor, photo).angle_tenths, 3);
    assert_eq!(editor.set_angle_text("−12,04°"), Ok(true));
    assert_eq!(adjust_of(&editor, photo).angle_tenths, -120);
    let before = editor.session().unwrap().history_list().steps.len();
    assert_eq!(editor.set_angle_text("abc"), Err(BAD_ANGLE));
    assert_eq!(adjust_of(&editor, photo).angle_tenths, -120);
    assert_eq!(editor.session().unwrap().history_list().steps.len(), before);
}

#[test]
fn af_b9_os_ajustes_seguem_a_foto() {
    let (mut editor, _, photo) = selected();
    assert!(editor.adjust(AdjustChange::Angle(20)));
    assert!(editor.adjust(AdjustChange::Bw(true)));
    let wanted = adjust_of(&editor, photo);
    // Trocar o layout, espelhar e pôr a foto em outro quadro: o ajuste continua com a foto.
    assert!(editor.run(Command::NextLayout));
    assert!(editor.run(Command::Mirror));
    let other = photo_frames(&editor)
        .into_iter()
        .find(|(_, p)| *p != photo)
        .unwrap()
        .0;
    assert!(editor.drop_photo(other, photo));
    assert_eq!(adjust_of(&editor, photo), wanted);
    // A mesma foto em dois quadros: os dois leem o mesmo ajuste, da foto.
    let uses = photo_frames(&editor)
        .iter()
        .filter(|(_, p)| *p == photo)
        .count();
    assert!(uses >= 1);
    assert_eq!(wanted.angle_tenths, 20);
    assert!(wanted.bw);
}

#[test]
fn af_b10_um_gesto_uma_edicao() {
    let (mut editor, _, photo) = selected();
    let before = labels(&editor).len();
    for step in 1..=20 {
        assert!(editor.adjust(AdjustChange::Brightness(step * 3)));
    }
    editor.end_adjust();
    assert_eq!(labels(&editor).len(), before + 1);
    assert_eq!(labels(&editor).last().unwrap(), "Brilho");
    assert_eq!(adjust_of(&editor, photo).brightness, 60);
    assert!(editor.run(Command::Undo));
    assert_eq!(adjust_of(&editor, photo), PhotoAdjust::default());
    // Depois do fim do arrasto, outro arrasto é outra edição.
    assert!(editor.adjust(AdjustChange::Contrast(-10)));
    editor.end_adjust();
    assert!(editor.adjust(AdjustChange::Contrast(-20)));
    assert_eq!(labels(&editor).last().unwrap(), "Contraste");
}

#[test]
fn af_b11_teclas_no_recorte() {
    let (mut editor, frame, photo) = selected();
    let key = |c| Shortcut::key(Key::Char(c));
    // Fora do recorte as teclas ficam indisponíveis.
    assert!(!editor.available(Command::TurnRight));
    assert!(editor.enter_crop(frame));
    for _ in 0..3 {
        let c = Command::from_shortcut(key(']')).unwrap();
        assert!(editor.run(c));
    }
    // Shift+[ chega como "{" (o Shift do teclado); o Shift sozinho não conta nos sinais.
    let shifted = Shortcut {
        shift: true,
        ..key('{')
    };
    let c = Command::from_shortcut(shifted).unwrap();
    assert_eq!(c, Command::TurnLeftMore);
    assert!(editor.run(c));
    assert_eq!(adjust_of(&editor, photo).angle_tenths, -7);
    // Nenhum atalho se repete.
    let mut seen = std::collections::HashSet::new();
    for c in Command::ALL {
        if let Some(s) = c.shortcut()
            && c.in_mode(crate::commands::Mode::Normal)
        {
            assert!(seen.insert(s), "{c:?} repete {s:?}");
        }
    }
}

#[test]
fn af_b12_grade_do_recorte() {
    let (mut editor, frame, _) = selected();
    assert_eq!(
        editor.crop_grid(),
        CropGrid::default(),
        "fora do recorte, nada"
    );
    assert!(editor.enter_crop(frame));
    assert_eq!(
        editor.crop_grid(),
        CropGrid {
            thirds: true,
            fine: false
        }
    );
    assert!(editor.run(Command::TurnRight));
    assert!(
        editor.crop_grid().fine,
        "a grade fina enquanto o ângulo muda"
    );
    editor.angle_settled();
    assert!(!editor.crop_grid().fine);
    // A grade é só da tela: o projeto não guarda nada dela.
    let json = editor.session().unwrap().project().to_json().unwrap();
    assert!(!json.contains("grade") && !json.contains("grid"));
}

#[test]
fn af_b17_restaurar_a_foto() {
    let (mut editor, _, photo) = selected();
    for change in [
        AdjustChange::Angle(30),
        AdjustChange::Brightness(20),
        AdjustChange::Contrast(-10),
        AdjustChange::Bw(true),
    ] {
        assert!(editor.adjust(change));
        editor.end_adjust();
    }
    let before = labels(&editor).len();
    assert!(editor.adjust(AdjustChange::Restore));
    assert_eq!(adjust_of(&editor, photo), PhotoAdjust::default());
    assert_eq!(labels(&editor).len(), before + 1);
    assert_eq!(labels(&editor).last().unwrap(), "Restaurar ajustes");
    assert!(
        !editor.adjust(AdjustChange::Restore),
        "já neutro: nada muda"
    );
}

#[test]
fn af_b18_escala_por_cima_da_cobertura() {
    let (mut editor, frame, photo) = selected();
    assert!(editor.adjust(AdjustChange::Angle(50)));
    assert_eq!(editor.photo_scale(), Some(100.0));
    assert_eq!(editor.set_scale_text("150%"), Ok(true));
    let s = editor.session().unwrap();
    let f = &s.project().spreads[1].frames[frame];
    let zoom = match &f.content {
        FrameContent::Photo { crop, .. } => crop.zoom,
        _ => 0.0,
    };
    assert!((zoom - 1.5).abs() < 1e-9, "zoom {zoom}");
    assert_eq!(adjust_of(&editor, photo).angle_tenths, 50);
    assert_eq!(editor.set_scale_text("doze"), Err(BAD_SCALE));
}

#[test]
fn af_b19_varios_quadros() {
    let mut editor = editor_at(three_spreads(), 1);
    let frames = photo_frames(&editor);
    assert!(frames.len() >= 3);
    assert!(editor.run(Command::Organize));
    for (k, (f, _)) in frames.iter().take(3).enumerate() {
        editor.organize_click(*f, k > 0, false);
    }
    let before = labels(&editor).len();
    assert!(editor.adjust(AdjustChange::Bw(true)));
    assert_eq!(labels(&editor).len(), before + 1, "uma edição para as três");
    for (_, p) in frames.iter().take(3) {
        assert!(adjust_of(&editor, *p).bw);
    }
    assert!(
        !editor.can_adjust(AdjustChange::Angle(10)),
        "o ângulo pede um quadro só"
    );
    assert!(!editor.adjust(AdjustChange::Angle(10)));
}

#[test]
fn af_b8_a_tela_desenha_pela_funcao_da_exportacao() {
    let (mut editor, frame, _) = selected();
    let screen: Vec<u8> = (0..300 * 200)
        .flat_map(|i: u32| [(i % 251) as u8, (i % 113) as u8, 90, 255])
        .collect();
    let per_mm = 2.0;
    let project = editor.session().unwrap().project().clone();
    assert!(
        crate::adjusted::adjusted_frame(&project, (1, frame), (300, 200, &screen), per_mm)
            .is_none(),
        "sem ajuste, a tela recorta a foto inteira como hoje"
    );
    assert!(editor.adjust(AdjustChange::Bw(true)));
    assert!(editor.adjust(AdjustChange::Angle(35)));
    let project = editor.session().unwrap().project().clone();
    let drawn =
        crate::adjusted::adjusted_frame(&project, (1, frame), (300, 200, &screen), per_mm).unwrap();
    let f = &project.spreads[1].frames[frame];
    let bled = lamina_render::bleed(&project.spec, project.frame_rect(f));
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let want = (
        (bled.rect.width.get() * per_mm).round() as u32,
        (bled.rect.height.get() * per_mm).round() as u32,
    );
    assert_eq!((drawn.width, drawn.height), want);
    assert_eq!(drawn.rgba.len(), (want.0 * want.1 * 4) as usize);
    assert!(drawn.rgba.chunks(4).all(|p| p[0] == p[1] && p[1] == p[2]));
}
