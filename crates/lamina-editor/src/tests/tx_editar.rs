//! Cenários da spec `texto-e-quadros` (TX) sobre editar um texto que já está na lâmina (correção
//! de 2026-10-28): B17 a B21.

use std::path::Path;

use lamina_core::{FrameContent, PageSide};

use super::eg_imas::album_mm;
use super::tl_common::{editor_at, three_spreads};
use crate::editor::{ContextEntry, DoubleClick, TextFocus};
use crate::hand::NEW_TEXT_HEIGHT_MM;
use crate::snap::Grip;
use crate::{Area, Command, Editor, FrameKind, Install, StripsStore, layers, spread_view};

/// A lâmina 1 de três, com o painel da direita escondido e salvo assim.
fn hidden_panel(dir: &Path) -> Editor {
    let file = dir.join("janela.json");
    let text = r#"{"versao": 2, "laminas": false, "linha_do_tempo": true, "fotos": true, "painel": false}"#;
    std::fs::write(&file, text).unwrap();
    let mut editor = editor_at(three_spreads(), 0);
    editor.set_strips(StripsStore::load_for(&file, Install::Existing));
    assert!(!editor.panel_shown());
    editor
}

/// Apertar e soltar o mouse num quadro, sem arrastar.
fn click(editor: &mut Editor, frame: usize) {
    assert!(editor.frame_grab(frame, Grip::Move));
    editor.frame_release();
}

fn first_photo(editor: &Editor) -> usize {
    let s = editor.session().unwrap();
    s.project().spreads[s.current()]
        .frames
        .iter()
        .position(|f| matches!(f.content, FrameContent::Photo { .. }))
        .unwrap()
}

#[test]
fn tx_b17_clique_num_texto_antigo_mostra_o_campo() {
    let dir = tempfile::tempdir().unwrap();
    let mut editor = hidden_panel(dir.path());
    assert!(editor.new_text(Some(PageSide::Left)));
    let first = editor.selected_frame().unwrap();
    assert!(editor.new_text(Some(PageSide::Right)));
    let second = editor.selected_frame().unwrap();
    assert_eq!(
        editor.take_text_focus(),
        Some(TextFocus::Cursor),
        "T põe o cursor"
    );
    assert!(editor.run(Command::Deselect));
    assert!(!editor.panel_shown());

    click(&mut editor, first);
    assert!(editor.panel_shown(), "clicar num texto mostra o painel");
    assert_eq!(
        editor.take_text_focus(),
        Some(TextFocus::Tab),
        "na aba Quadro, sem tirar o teclado da lâmina"
    );
    click(&mut editor, second);
    assert_eq!(editor.selected_frame(), Some(second));
    assert!(
        editor.panel_shown(),
        "de um texto para outro, o campo continua"
    );
    assert_eq!(editor.take_text_focus(), Some(TextFocus::Tab));

    let photo = first_photo(&editor);
    click(&mut editor, photo);
    assert!(!editor.panel_shown(), "clicar numa foto encerra o pedido");
    assert_eq!(editor.take_text_focus(), None);
}

#[test]
fn tx_b18_texto_fica_por_cima_das_fotos_na_tela() {
    let mut editor = editor_at(three_spreads(), 0);
    assert!(editor.new_text(Some(PageSide::Left)));
    // Trocar o layout guarda o texto e põe as fotos novas depois dele na lista.
    assert!(editor.run(Command::NextLayout));
    let project = editor.session().unwrap().project();
    let area = Area {
        width: 1200.0,
        height: 700.0,
        margin: 10.0,
    };
    let view = spread_view(project, 0, area, &[]).unwrap();
    let text = view
        .frames
        .iter()
        .position(|f| matches!(f.kind, FrameKind::Text { .. }))
        .unwrap();
    assert!(
        text + 1 < view.frames.len(),
        "há fotos depois do texto na lista"
    );
    let layer = layers(&view.frames);
    for (i, f) in view.frames.iter().enumerate() {
        if matches!(f.kind, FrameKind::Photo { .. }) {
            assert!(layer[text] > layer[i], "o texto fica por cima da foto {i}");
        }
    }
    let mut sorted = layer.clone();
    sorted.sort_unstable();
    assert_eq!(
        sorted,
        (0..layer.len()).collect::<Vec<_>>(),
        "uma camada por quadro"
    );
}

#[test]
fn tx_b19_duplo_clique_num_texto_no_modo_organizar() {
    let mut editor = editor_at(three_spreads(), 0);
    assert!(editor.new_text(Some(PageSide::Right)));
    let text = editor.selected_frame().unwrap();
    editor.take_text_focus();
    assert!(editor.run(Command::Organize));
    assert_eq!(editor.double_click(Some(text)), DoubleClick::Text);
    assert!(
        !editor.organizing(),
        "sai do modo, onde não há campo de texto"
    );
    assert_eq!(
        editor.selected_frame(),
        Some(text),
        "o texto continua selecionado"
    );
    assert!(editor.panel_shown());
    assert_eq!(editor.take_text_focus(), Some(TextFocus::Cursor));
}

/// O ponto (`x`, `y`) mm do corte em frações da largura da lâmina com a sangria (a paleta).
fn at(editor: &Editor, x: f64, y: f64) -> (f64, f64) {
    let spec = &editor.session().unwrap().project().spec;
    let (canvas, trim) = (spec.canvas_rect(), spec.trim_rect());
    let w = canvas.width.get();
    (
        (trim.x.get() + x - canvas.x.get()) / w,
        (trim.y.get() + y - canvas.y.get()) / w,
    )
}

#[test]
fn tx_b20_clique_da_ferramenta_texto() {
    let project = album_mm(&[(20.0, 40.0, 40.0, 50.0)]);
    let mut editor = editor_at(project, 1);
    assert!(editor.new_text(Some(PageSide::Right)));
    let text = editor.selected_frame().unwrap();
    let r = editor.frames_mm()[text];
    let trim = editor.session().unwrap().project().spec.trim_rect();
    let middle = at(
        &editor,
        r.x.get() + r.width.get() / 2.0 - trim.x.get(),
        r.y.get() + r.height.get() / 2.0 - trim.y.get(),
    );
    assert!(editor.run(Command::Organize));
    let before = editor.frames_mm().len();
    editor.take_text_focus();

    assert!(!editor.draw_frame(true, middle, middle), "não cria nada");
    assert_eq!(editor.frames_mm().len(), before);
    assert_eq!(
        editor.selected_frame(),
        Some(text),
        "abre o texto que estava ali"
    );
    assert!(!editor.organizing());
    assert_eq!(editor.take_text_focus(), Some(TextFocus::Cursor));

    // No vazio (no alto da página direita): um texto no tamanho do T, centrado no clique.
    assert!(editor.run(Command::Organize));
    let page = editor
        .session()
        .unwrap()
        .project()
        .spec
        .page_rect(PageSide::Right);
    let (cx, cy) = (page.x.get() + page.width.get() / 2.0, page.y.get() + 30.0);
    let empty = at(&editor, cx - trim.x.get(), cy - trim.y.get());
    assert!(editor.draw_frame(true, empty, empty));
    let frames = editor.frames_mm();
    assert_eq!(frames.len(), before + 1);
    let new = frames[before];
    assert!((new.width.get() - page.width.get() * 0.6).abs() < 1e-6);
    assert!((new.height.get() - NEW_TEXT_HEIGHT_MM).abs() < 1e-6);
    assert!((new.x.get() + new.width.get() / 2.0 - cx).abs() < 1e-6);
    assert!((new.y.get() + new.height.get() / 2.0 - cy).abs() < 1e-6);
    assert_eq!(
        editor.take_text_focus(),
        Some(TextFocus::Cursor),
        "o texto novo pede o cursor"
    );
}

#[test]
fn tx_b21_editar_texto_no_menu_de_contexto() {
    let dir = tempfile::tempdir().unwrap();
    let mut editor = hidden_panel(dir.path());
    assert!(editor.new_text(Some(PageSide::Left)));
    let text = editor.selected_frame().unwrap();
    assert!(editor.run(Command::Deselect));
    editor.take_text_focus();
    let enabled = |menu: &[ContextEntry]| {
        menu.iter().any(|e| {
            matches!(
                e,
                ContextEntry::Item {
                    command: Command::EditText,
                    enabled: true,
                    ..
                }
            )
        })
    };
    let photo = first_photo(&editor);
    assert!(
        !enabled(&editor.context_menu(Some(photo))),
        "numa foto, apagado"
    );
    assert!(
        enabled(&editor.context_menu(Some(text))),
        "num texto, aceso"
    );
    assert!(editor.run(Command::EditText));
    assert!(editor.panel_shown());
    assert_eq!(editor.take_text_focus(), Some(TextFocus::Cursor));
}
