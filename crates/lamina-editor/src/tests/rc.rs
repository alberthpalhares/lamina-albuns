//! Cenários da spec `recorte` (RC-B1 a B10).

use std::time::{Duration, Instant};

use lamina_core::testing::{TRIES, budget};
use lamina_core::{Crop, Dpi, FrameContent, Mm, Project, RelRect};
use lamina_render::{bleed, region};

use super::common::isolated;
use super::tl_common::{H, V, built, editor_at, spec, three_spreads};
use crate::view::Area;
use crate::{Command, Editor, FrameKind, Key, Shortcut, frame_dpi_text, photo_extent, spread_view};

fn crop_of(editor: &Editor, frame: usize) -> Crop {
    let s = editor.session().unwrap();
    match &s.project().spreads[s.current()].frames[frame].content {
        FrameContent::Photo { crop, .. } => *crop,
        _ => panic!("quadro sem foto"),
    }
}

fn shown_crop(editor: &Editor, frame: usize) -> Crop {
    let p = editor.view_project().unwrap();
    let index = editor.session().unwrap().current();
    match &p.spreads[index].frames[frame].content {
        FrameContent::Photo { crop, .. } => *crop,
        _ => panic!("quadro sem foto"),
    }
}

fn area(project: &Project) -> Area {
    let c = project.spec.canvas_rect();
    Area {
        width: 1.0,
        height: c.height.get() / c.width.get(),
        margin: 0.0,
    }
}

#[test]
fn rc_b1_entrar_e_sair_do_modo_recorte() {
    let mut editor = editor_at(three_spreads(), 1);
    editor.select_frame(Some(0));
    assert!(editor.run(Command::CropMode));
    assert_eq!(editor.crop_mode(), Some(0));
    assert!(editor.run(Command::Deselect));
    assert_eq!(editor.crop_mode(), None);
    editor.select_frame(Some(0));
    assert!(editor.run(Command::CropMode));
    editor.select_frame(Some(1));
    assert_eq!(
        (editor.crop_mode(), editor.selected_frame()),
        (None, Some(1))
    );
    assert!(editor.run(Command::ClearFrame));
    assert!(!editor.available(Command::CropMode), "quadro vazio");
}

#[test]
fn rc_b2_mover_a_foto() {
    let mut editor = editor_at(three_spreads(), 1);
    editor.select_frame(Some(0));
    assert!(editor.run(Command::CropMode));
    // Zoom 2: a foto passa do quadro nos dois sentidos e pode ser movida.
    assert!(editor.crop_zoom(2.0) && editor.commit_crop());
    let before = crop_of(&editor, 0);
    assert!(editor.crop_drag(0.1, 0.0));
    assert!(editor.commit_crop());
    let after = crop_of(&editor, 0);
    let s = editor.session().unwrap();
    let f = &s.project().spreads[1].frames[0];
    let FrameContent::Photo { photo, .. } = f.content else {
        panic!("quadro com foto")
    };
    let p = s.project().photo(photo).unwrap();
    let r = s.project().frame_rect(f);
    let (w, h, aspect) = (
        f64::from(p.width_px),
        f64::from(p.height_px),
        r.width.get() / r.height.get(),
    );
    assert_eq!(after, before.panned(w, h, aspect, 0.1, 0.0));
    assert!(
        after.center_x < before.center_x,
        "a parte visível foi para a esquerda"
    );
    assert!(editor.crop_drag(50.0, 0.0) && editor.commit_crop());
    let edge = crop_of(&editor, 0).visible_region(w, h, aspect);
    assert!(edge.x >= -1e-9, "nunca deixa espaço vazio: {edge:?}");
}

#[test]
fn rc_b3_zoom() {
    let mut editor = editor_at(three_spreads(), 1);
    editor.select_frame(Some(0));
    assert!(editor.run(Command::CropMode));
    for _ in 0..3 {
        assert!(editor.run(Command::ZoomIn));
    }
    assert!((crop_of(&editor, 0).zoom - 1.331).abs() < 1e-9);
    assert!(editor.run(Command::ZoomOut));
    assert!((crop_of(&editor, 0).zoom - 1.21).abs() < 1e-9);
    assert!(editor.run(Command::ResetCrop));
    assert_eq!(crop_of(&editor, 0), Crop::default());
    assert!(editor.crop_zoom(100.0) && editor.commit_crop());
    assert!((crop_of(&editor, 0).zoom - Crop::MAX_ZOOM).abs() < 1e-9);
    assert!(editor.crop_zoom(0.001) && editor.commit_crop());
    assert!((crop_of(&editor, 0).zoom - 1.0).abs() < 1e-9);
}

#[test]
fn rc_b4_um_gesto_uma_edicao() {
    let mut editor = editor_at(three_spreads(), 1);
    editor.select_frame(Some(0));
    assert!(editor.run(Command::CropMode));
    assert!(editor.crop_zoom(2.0) && editor.commit_crop());
    let start = crop_of(&editor, 0);
    let mut s = editor.session_mut().unwrap();
    assert!(s.undo().is_some());
    assert!(s.redo().is_some());
    for _ in 0..30 {
        assert!(editor.crop_drag(0.002, 0.001));
    }
    assert_eq!(
        crop_of(&editor, 0),
        start,
        "o histórico não muda durante o gesto"
    );
    assert_ne!(
        shown_crop(&editor, 0),
        start,
        "a lâmina mostra o provisório"
    );
    assert!(editor.commit_crop());
    s = editor.session_mut().unwrap();
    assert_eq!(s.undo().as_deref(), Some("Recortar foto · lâmina 2"));
    assert_eq!(crop_of(&editor, 0), start);
    assert!(editor.crop_drag(0.05, 0.0));
    editor.cancel_crop();
    assert_eq!(shown_crop(&editor, 0), start, "Esc descarta o provisório");
}

#[test]
fn rc_b5_visao_igual_a_impressao() {
    let mut project = three_spreads();
    for f in &mut project.spreads[1].frames {
        if let FrameContent::Photo { crop, .. } = &mut f.content {
            *crop = Crop {
                zoom: 2.0,
                center_x: 0.35,
                center_y: 0.6,
            };
        }
    }
    let view = spread_view(&project, 1, area(&project), &[]).unwrap();
    for (f, shown) in project.spreads[1].frames.iter().zip(&view.frames) {
        let FrameContent::Photo { photo, crop } = &f.content else {
            panic!("quadro com foto")
        };
        let p = project.photo(*photo).unwrap();
        let (pw, ph) = (f64::from(p.width_px), f64::from(p.height_px));
        let rect = project.frame_rect(f);
        let used = region(crop, rect, &bleed(&project.spec, rect), (pw, ph));
        let FrameKind::Photo { source, .. } = &shown.kind else {
            panic!("quadro com foto")
        };
        assert!(
            (source.x * pw - used.x).abs() < 1e-6 && (source.width * pw - used.width).abs() < 1e-6
        );
        assert!(
            (source.y * ph - used.y).abs() < 1e-6
                && (source.height * ph - used.height).abs() < 1e-6
        );
    }
}

#[test]
fn rc_b6_previa_do_recorte() {
    let mut editor = editor_at(three_spreads(), 1);
    editor.select_frame(Some(0));
    assert!(editor.run(Command::CropMode));
    assert!(editor.crop_zoom(2.0));
    let project = editor.view_project().unwrap().into_owned();
    let view = spread_view(&project, 1, area(&project), &[]).unwrap();
    let frame = &view.frames[0];
    let extent = photo_extent(frame).unwrap();
    let FrameKind::Photo { source, .. } = &frame.kind else {
        panic!("quadro com foto")
    };
    let px = 1000.0; // 1 px numa lâmina desenhada com 1000 px de largura
    assert!((extent.x + source.x * extent.width - frame.rect.x).abs() * px < 1.0);
    assert!((extent.y + source.y * extent.height - frame.rect.y).abs() * px < 1.0);
    assert!((source.width * extent.width - frame.rect.width).abs() * px < 1.0);
    assert!(
        extent.width > frame.rect.width,
        "a foto inteira é maior que o quadro"
    );
}

#[test]
fn rc_b7_dpi_efetivo_no_painel() {
    let spec =
        lamina_core::AlbumSpec::new(Mm::from_cm(30.0), Mm::from_cm(30.0), Dpi::new(300).unwrap())
            .unwrap();
    let mut project = Project::new("teste", spec);
    let photo = project.add_photo("f.jpg", 3000, 2000);
    let spread = project.add_spread();
    // 15 cm de largura numa lâmina de 60 cm; altura na proporção da foto.
    let rect = RelRect::new(0.1, 0.1, 0.25, 0.25 * 600.0 / 300.0 / 1.5).unwrap();
    project
        .add_frame(
            spread,
            rect,
            FrameContent::Photo {
                photo,
                crop: Crop::default(),
            },
        )
        .unwrap();
    let dpi = project
        .effective_dpi(&project.spreads[0].frames[0])
        .unwrap()
        .unwrap();
    let (text, warn) = frame_dpi_text(&project, 0, 0).unwrap();
    let percent = (300.0 / dpi * 100.0).round();
    assert_eq!(
        text,
        format!("DPI efetivo {} · ampliada {percent}%", dpi.round())
    );
    assert!(!warn);
    let mut editor = editor_at(project, 0);
    editor.select_frame(Some(0));
    assert!(editor.run(Command::CropMode));
    assert!(editor.crop_zoom(2.0));
    let shown = editor.view_project().unwrap().into_owned();
    let (text2, _) = frame_dpi_text(&shown, 0, 0).unwrap();
    assert_eq!(
        text2,
        format!(
            "DPI efetivo {} · ampliada {}%",
            (dpi / 2.0).round(),
            (600.0 / dpi * 100.0).round()
        )
    );
    assert!(editor.crop_zoom(1.6));
    let (_, warn) = frame_dpi_text(&editor.view_project().unwrap(), 0, 0).unwrap();
    assert!(warn, "acima de 120% vira aviso");
}

#[test]
fn rc_b8_recorte_mantido_ao_trocar() {
    let mut project = three_spreads();
    let kept = Crop {
        zoom: 2.0,
        center_x: 0.3,
        center_y: 0.6,
    };
    let FrameContent::Photo { photo, crop } = &mut project.spreads[1].frames[0].content else {
        panic!("quadro com foto")
    };
    *crop = kept;
    let id = *photo;
    let mut editor = editor_at(project, 1);
    let crop_of_photo = |e: &Editor| {
        let s = e.session().unwrap();
        s.project().spreads[1]
            .frames
            .iter()
            .find_map(|f| match &f.content {
                FrameContent::Photo { photo, crop } if *photo == id => Some(*crop),
                _ => None,
            })
            .unwrap()
    };
    assert!(editor.run(Command::NextLayout));
    let expect = Crop { zoom: 1.0, ..kept };
    assert_eq!(crop_of_photo(&editor), expect);
    let at = editor.session().unwrap().project().spreads[1]
        .frames
        .iter()
        .position(|f| matches!(&f.content, FrameContent::Photo { photo, .. } if *photo == id))
        .unwrap();
    assert!(editor.drop_frame(at, usize::from(at == 0)));
    assert_eq!(crop_of_photo(&editor), expect);
    let other = editor.session().unwrap().project().photos[0].id;
    assert!(editor.drop_photo(0, other));
    assert_eq!(
        crop_of(&editor, 0),
        Crop::default(),
        "foto nova: recorte padrão"
    );
}

#[test]
fn rc_b9_registro_de_comandos() {
    let shortcuts: Vec<Shortcut> = Command::ALL.iter().filter_map(|c| c.shortcut()).collect();
    let unique: std::collections::HashSet<_> = shortcuts.iter().collect();
    assert_eq!(unique.len(), shortcuts.len(), "atalhos repetidos");
    let plus = Shortcut {
        ctrl: false,
        shift: true,
        alt: false,
        key: Key::Char('+'),
    };
    assert_eq!(Command::from_shortcut(plus), Some(Command::ZoomIn));
    assert_eq!(
        Command::from_shortcut(Shortcut::key(Key::Char('='))),
        Some(Command::ZoomIn)
    );
    let mut editor = editor_at(three_spreads(), 1);
    assert!(
        !editor.available(Command::CropMode),
        "sem quadro selecionado"
    );
    editor.select_frame(Some(0));
    assert!(editor.available(Command::CropMode));
    for command in [Command::ZoomIn, Command::ZoomOut, Command::ResetCrop] {
        assert!(
            !editor.available(command),
            "{} fora do modo recorte",
            command.name()
        );
    }
}

#[test]
fn rc_b10_recortar_rapido() {
    let photos: Vec<(u32, u32)> = (0..40).map(|i| if i % 4 == 3 { V } else { H }).collect();
    let project = built(spec(30.0, 30.0), &photos, 10);
    let (mut step, mut save) = (Duration::ZERO, Duration::MAX);
    for _ in 0..TRIES {
        let mut editor = editor_at(project.clone(), 1);
        editor.select_frame(Some(0));
        assert!(editor.run(Command::CropMode) && editor.crop_zoom(2.0));
        let (worst, saved) = isolated(|| {
            let mut worst = Duration::ZERO;
            for _ in 0..30 {
                let started = Instant::now();
                assert!(editor.crop_drag(0.003, 0.002));
                let shown = editor.view_project().unwrap();
                assert!(spread_view(&shown, 1, area(&shown), &[]).is_some());
                worst = worst.max(started.elapsed());
            }
            let started = Instant::now();
            assert!(editor.commit_crop());
            (worst, started.elapsed())
        });
        step = if step == Duration::ZERO {
            worst
        } else {
            step.min(worst)
        };
        save = save.min(saved);
        if step < budget(16) && save < budget(50) {
            break;
        }
    }
    assert!(step < budget(16), "passo do arrasto em {step:?}");
    assert!(save < budget(50), "gravar em {save:?}");
}
