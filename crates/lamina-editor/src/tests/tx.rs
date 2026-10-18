//! Cenários da spec `texto-e-quadros` (TX) no editor: B1, B3, B6 a B12 e B14. Os de desenho e
//! de avisos (B2, B4, B5, B13, B15, B16) moram no renderizador e na exportação.

use lamina_core::{FrameContent, Mm, PageSide, Project, Rect, Rgb, TextAlign, TextBox, TextVAlign};

use super::tl_common::{editor_at, spec, three_spreads};
use crate::hand::frame_mm;
use crate::keys::{Key, Shortcut};
use crate::snap::Grip;
use crate::{Command, Editor, Session, TextChange};

fn current(editor: &Editor) -> (&Project, usize) {
    let s = editor.session().unwrap();
    (s.project(), s.current())
}

fn rect_of(editor: &Editor, frame: usize) -> Rect {
    let (p, i) = current(editor);
    frame_mm(p, i, frame).unwrap()
}

fn close(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-6
}

fn undo_label(editor: &Editor) -> String {
    editor.session().unwrap().undo_label().unwrap().to_owned()
}

#[test]
fn tx_b1_novo_texto() {
    let mut editor = editor_at(three_spreads(), 1);
    assert_eq!(
        Command::from_shortcut(Shortcut::key(Key::Char('t'))),
        Some(Command::NewText)
    );
    assert!(editor.new_text(Some(PageSide::Right)));
    let frame = editor.selected_frame().unwrap();
    let text = editor.selected_text().unwrap().clone();
    assert_eq!(text.text, "");
    assert_eq!(text.font, "Inter");
    assert!(close(text.size_pt, 24.0));
    assert_eq!(text.align, TextAlign::Center);
    assert_eq!(
        text.color,
        Rgb {
            r: 0x1A,
            g: 0x1A,
            b: 0x1A
        }
    );
    let (p, i) = current(&editor);
    let page = p.spec.page_rect(PageSide::Right);
    let r = rect_of(&editor, frame);
    assert!(close(
        r.x.get() + r.width.get() / 2.0,
        page.x.get() + page.width.get() / 2.0
    ));
    assert!(close(r.width.get(), page.width.get() * 0.6));
    assert!(!p.spreads[i].manual);
    assert_eq!(undo_label(&editor), "Novo texto");
    // Fundo escuro: texto branco.
    editor
        .session_mut()
        .unwrap()
        .edit("fundo", |p: &mut Project| {
            p.spreads[1].background = Rgb::BLACK;
            Ok::<(), ()>(())
        })
        .unwrap();
    assert!(editor.new_text(Some(PageSide::Left)));
    assert_eq!(editor.selected_text().unwrap().color, Rgb::WHITE);
}

#[test]
fn tx_b3_os_campos_do_texto_ficam_no_projeto() {
    let mut editor = editor_at(three_spreads(), 0);
    editor.new_text(None);
    for change in [
        TextChange::Text("Ana & Pedro".into()),
        TextChange::Font("Playfair Display".into()),
        TextChange::Size(36.0),
        TextChange::Bold(true),
        TextChange::Italic(true),
        TextChange::Color(Rgb {
            r: 0x8A,
            g: 0x1C,
            b: 0x1C,
        }),
        TextChange::Align(TextAlign::Right),
        TextChange::VAlign(TextVAlign::Bottom),
        TextChange::LineSpacing(1.5),
    ] {
        assert!(editor.change_text(change.clone()), "{change:?}");
    }
    assert!(
        !editor.change_text(TextChange::Size(500.0)),
        "fora da faixa"
    );
    let text = editor.selected_text().unwrap().clone();
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("album.lamina");
    editor.session_mut().unwrap().save_as(&file).unwrap();
    let opened = Session::open(&file).unwrap();
    let again = opened.project().spreads[0]
        .frames
        .iter()
        .find_map(|f| match &f.content {
            FrameContent::Text(t) => Some(t.clone()),
            _ => None,
        })
        .unwrap();
    assert_eq!(again, text);
    // Projeto v3 antigo, só com texto, tamanho e cor: abre com os padrões.
    let mut json: serde_json::Value =
        serde_json::from_str(&opened.project().to_json().unwrap()).unwrap();
    let frames = json["spreads"][0]["frames"].as_array_mut().unwrap();
    let t = frames
        .iter_mut()
        .find(|f| f["content"]["kind"] == "text")
        .unwrap();
    for key in ["font", "bold", "italic", "align", "valign", "line_spacing"] {
        t["content"].as_object_mut().unwrap().remove(key);
    }
    let old = Project::from_json(&json.to_string()).unwrap();
    let back = old.spreads[0].frames.iter().find_map(|f| match &f.content {
        FrameContent::Text(t) => Some(t.clone()),
        _ => None,
    });
    let defaults = TextBox {
        text: "Ana & Pedro".into(),
        size_pt: 36.0,
        color: Rgb {
            r: 0x8A,
            g: 0x1C,
            b: 0x1C,
        },
        ..TextBox::new(Rgb::BLACK)
    };
    assert_eq!(back, Some(defaults));
}

#[test]
fn tx_b6_mover_e_redimensionar_um_quadro_de_foto() {
    let mut editor = editor_at(three_spreads(), 2);
    let before = editor.session().unwrap().project().clone();
    let start = rect_of(&editor, 0);
    let crop = before.spreads[2].frames[0].content.clone();
    assert!(editor.frame_grab(0, Grip::Move));
    assert!(editor.frame_drag(20.0, 0.0, false, 0.0));
    assert!(editor.frame_release());
    let moved = rect_of(&editor, 0);
    assert!(close(moved.x.get(), start.x.get() + 20.0));
    let corner = Grip::Resize {
        left: false,
        right: true,
        top: false,
        bottom: true,
    };
    assert!(editor.frame_grab(0, corner));
    editor.frame_drag(-1000.0, -1000.0, false, 0.0);
    assert!(editor.frame_release());
    let small = rect_of(&editor, 0);
    assert!(close(small.width.get(), 5.0) && close(small.height.get(), 5.0));
    let (p, i) = current(&editor);
    assert!(p.spreads[i].manual);
    assert_eq!(
        p.spreads[i].frames[0].content, crop,
        "o recorte guarda o centro"
    );
    assert_eq!(undo_label(&editor), "Redimensionar quadro");
    assert!(editor.run(Command::Undo));
    assert_eq!(undo_label(&editor), "Mover quadro");
    assert!(editor.run(Command::Undo));
    assert_eq!(editor.session().unwrap().project(), &before);
}

fn empty_spread() -> Editor {
    let mut project = Project::new("teste", spec(30.0, 30.0));
    project.add_spread();
    editor_at(project, 0)
}

#[test]
fn tx_b7_ima() {
    let mut editor = empty_spread();
    editor.new_text(Some(PageSide::Left));
    let start = rect_of(&editor, 0);
    let safe = editor
        .session()
        .unwrap()
        .project()
        .spec
        .safe_rect(PageSide::Left);
    // A borda esquerda fica a 1 mm da linha da zona segura (8 px da tela ≈ 1,5 mm aqui).
    let dx = safe.x.get() + 1.0 - start.x.get();
    editor.frame_grab(0, Grip::Move);
    editor.frame_drag(dx, 0.0, true, 1.5);
    assert!(
        editor
            .snap_lines()
            .iter()
            .any(|l| l.vertical && close(l.at, safe.x.get()))
    );
    editor.frame_release();
    assert!(close(rect_of(&editor, 0).x.get(), safe.x.get()));
    // Com Alt (sem ímã), fica onde soltou.
    editor.frame_grab(0, Grip::Move);
    editor.frame_drag(1.0, 0.0, false, 1.5);
    editor.frame_release();
    assert!(close(rect_of(&editor, 0).x.get(), safe.x.get() + 1.0));
}

#[test]
fn tx_b8_alt_setas() {
    let mut editor = empty_spread();
    editor.new_text(None);
    let start = rect_of(&editor, 0);
    let alt = |key, shift| Shortcut {
        ctrl: false,
        shift,
        alt: true,
        key,
    };
    for _ in 0..3 {
        assert!(editor.nudge_key(alt(Key::Right, false)));
    }
    assert!(editor.nudge_key(alt(Key::Down, true)));
    let r = rect_of(&editor, 0);
    assert!(close(r.x.get(), start.x.get() + 3.0) && close(r.y.get(), start.y.get() + 10.0));
    assert_eq!(undo_label(&editor), "Mover quadro");
    assert!(editor.run(Command::Undo));
    assert_eq!(undo_label(&editor), "Novo texto", "uma edição só");
    assert!(
        !editor.nudge_key(Shortcut::key(Key::Right)),
        "→ sem Alt é do layout"
    );
}

/// Lâmina 1 à mão (o 1º quadro movido) com dois textos.
fn manual_with_texts() -> Editor {
    let mut editor = editor_at(three_spreads(), 0);
    editor.new_text(Some(PageSide::Left));
    editor.change_text(TextChange::Text("Ana".into()));
    editor.new_text(Some(PageSide::Right));
    editor.change_text(TextChange::Text("Pedro".into()));
    editor.frame_grab(0, Grip::Move);
    editor.frame_drag(4.0, 4.0, false, 0.0);
    editor.frame_release();
    editor
}

fn texts(editor: &Editor) -> Vec<(String, lamina_core::RelRect, f64)> {
    let (p, i) = current(editor);
    p.spreads[i]
        .frames
        .iter()
        .filter_map(|f| match &f.content {
            FrameContent::Text(t) => Some((t.text.clone(), f.rect, t.size_pt)),
            _ => None,
        })
        .collect()
}

#[test]
fn tx_b9_trocar_o_layout_mantem_os_textos() {
    let mut editor = manual_with_texts();
    let before = texts(&editor);
    assert_eq!(before.len(), 2);
    assert!(editor.run(Command::NextLayout));
    assert_eq!(texts(&editor), before);
    let (p, i) = current(&editor);
    assert!(!p.spreads[i].manual);
}

#[test]
fn tx_b10_lamina_a_mao_no_espaco_e_na_especificacao() {
    let editor = manual_with_texts();
    let mut project = editor.session().unwrap().project().clone();
    let rects: Vec<_> = project.spreads[0].frames.iter().map(|f| f.rect).collect();
    let centers: Vec<_> = project.spreads[0]
        .frames
        .iter()
        .filter_map(|f| match &f.content {
            FrameContent::Photo { crop, .. } => Some((crop.center_x, crop.center_y)),
            _ => None,
        })
        .collect();
    crate::appearance::regap(&mut project, 0, Mm(8.0)).unwrap();
    assert_eq!(
        project.spreads[0]
            .frames
            .iter()
            .map(|f| f.rect)
            .collect::<Vec<_>>(),
        rects
    );
    crate::respec::respec(&mut project, spec(25.0, 25.0)).unwrap();
    let spread = &project.spreads[0];
    assert!(spread.manual);
    assert_eq!(
        spread.frames.iter().map(|f| f.rect).collect::<Vec<_>>(),
        rects
    );
    let after: Vec<_> = spread
        .frames
        .iter()
        .filter_map(|f| match &f.content {
            FrameContent::Photo { crop, .. } => Some((crop.center_x, crop.center_y)),
            FrameContent::Text(t) => {
                assert!(close(t.size_pt, 24.0));
                None
            }
            FrameContent::Empty => None,
        })
        .collect();
    assert_eq!(after, centers);
}

#[test]
fn tx_b11_delete_e_novo_quadro_de_foto() {
    let mut editor = editor_at(three_spreads(), 0);
    let n = current(&editor).0.spreads[0].frames.len();
    editor.select_frame(Some(0));
    assert!(editor.run(Command::ClearFrame));
    assert_eq!(
        current(&editor).0.spreads[0].frames[0].content,
        FrameContent::Empty
    );
    assert!(editor.run(Command::ClearFrame));
    let (p, _) = current(&editor);
    assert_eq!(p.spreads[0].frames.len(), n - 1);
    assert!(p.spreads[0].manual);
    assert!(editor.run(Command::NewPhotoFrame));
    let frame = editor.selected_frame().unwrap();
    let (p, _) = current(&editor);
    assert_eq!(p.spreads[0].frames[frame].content, FrameContent::Empty);
    let page = p.spec.page_rect(PageSide::Left);
    assert!(close(
        rect_of(&editor, frame).width.get(),
        page.width.get() / 3.0
    ));
    // Delete num texto apaga o texto.
    editor.new_text(None);
    assert!(editor.run(Command::ClearFrame));
    assert!(texts(&editor).is_empty());
}

#[test]
fn tx_b12_ordem_dos_quadros() {
    let mut editor = editor_at(three_spreads(), 0);
    editor.new_text(None);
    let (p, _) = current(&editor);
    let ids: Vec<_> = p.spreads[0].frames.iter().map(|f| f.id).collect();
    let last_photo = ids.len() - 2;
    editor.select_frame(Some(last_photo));
    assert!(editor.run(Command::SendBackward));
    let (p, _) = current(&editor);
    let now: Vec<_> = p.spreads[0].frames.iter().map(|f| f.id).collect();
    assert_eq!(now[last_photo - 1], ids[last_photo]);
    assert_eq!(now[last_photo], ids[last_photo - 1]);
    assert_eq!(now.last(), ids.last(), "o texto continua por cima");
    assert_eq!(undo_label(&editor), "Ordem dos quadros");
    // Texto não muda de ordem.
    editor.select_frame(Some(ids.len() - 1));
    assert!(!editor.available(Command::BringForward));
}

#[test]
fn tx_b14_fonte_que_nao_existe_fica_guardada() {
    let mut editor = empty_spread();
    editor.new_text(None);
    editor.change_text(TextChange::Font("Lobster".into()));
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("album.lamina");
    editor.session_mut().unwrap().save_as(&file).unwrap();
    let opened = Session::open(&file).unwrap();
    let font = opened.project().spreads[0]
        .frames
        .iter()
        .find_map(|f| match &f.content {
            FrameContent::Text(t) => Some(t.font.clone()),
            _ => None,
        });
    assert_eq!(font.as_deref(), Some("Lobster"));
}
