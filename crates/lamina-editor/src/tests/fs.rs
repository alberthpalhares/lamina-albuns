//! Cenários da spec `fontes-do-computador` (FS) no editor: a lista, a busca, a escolha, os
//! avisos, a IA, a trava, a leitura em segundo plano e as prévias (B1 a B4, B6 a B9, B11 a B14 e
//! B16). Os de desenho e de exportação moram no renderizador e na exportação.

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use lamina_core::testing::{TRIES, budget};
use lamina_core::{FrameContent, Project, RelRect, Rgb, TextBox};
use lamina_render::testing::{FONT_A, FONT_B, TestFonts};
use lamina_render::{FAMILIES, font_preview, installed_names, installed_names_ready};
use serde_json::json;

use super::common::Base;
use super::ia_common::Lab;
use super::tl_common::{editor_at, spec};
use crate::agent::ToolError;
use crate::font_picker::{
    COMPUTER, FontPreviews, FontRow, LAMINA, LOADING, PREVIEW_PX, font_rows, missing_fonts,
    preview, set_system_fonts_allowed,
};
use crate::{Command, Editor, TextChange};

/// As fontes de teste instaladas e a trava aberta.
fn fonts() -> TestFonts {
    let fonts = TestFonts::standard();
    set_system_fonts_allowed(true);
    fonts
}

/// A trava fechada enquanto existe (FS-B9).
struct Closed;

impl Closed {
    fn new() -> Self {
        set_system_fonts_allowed(false);
        Closed
    }
}

impl Drop for Closed {
    fn drop(&mut self) {
        set_system_fonts_allowed(true);
    }
}

/// Uma lâmina com um texto em cada fonte de `fonts`.
fn album(fonts: &[&str]) -> Project {
    let mut project = Project::new("teste", spec(30.0, 30.0));
    let spread = project.add_spread();
    for (i, font) in fonts.iter().enumerate() {
        #[allow(clippy::cast_precision_loss)]
        let y = 0.05 + 0.15 * i as f64;
        let text = TextBox {
            text: "Ana & Pedro".to_owned(),
            font: (*font).to_owned(),
            ..TextBox::new(Rgb::BLACK)
        };
        let rect = RelRect::new(0.55, y, 0.4, 0.12).unwrap();
        project
            .add_frame(spread, rect, FrameContent::Text(text))
            .unwrap();
    }
    project
}

/// O álbum gravado em `dir` e aberto num editor novo, como pelo menu Abrir.
fn opened(project: Project, dir: &Path) -> Editor {
    let file: PathBuf = dir.join("album.lamina");
    let mut saving = editor_at(project, 0);
    saving.session_mut().unwrap().save_as(&file).unwrap();
    let mut editor = Editor::new();
    editor.open(&file).unwrap();
    editor
}

fn names(rows: &[FontRow]) -> Vec<String> {
    rows.iter()
        .filter_map(|r| match r {
            FontRow::Font { name, .. } => Some(name.clone()),
            _ => None,
        })
        .collect()
}

fn font_of(editor: &Editor) -> String {
    editor.selected_text().unwrap().font.clone()
}

/// Espera a leitura em segundo plano terminar (no máximo 5 s).
fn wait_names() {
    let started = Instant::now();
    while installed_names_ready().is_none() && started.elapsed() < Duration::from_secs(5) {
        std::thread::yield_now();
    }
}

#[test]
fn fs_b1_a_lista_tem_as_do_lamina_e_as_do_computador() {
    let _fonts = fonts();
    installed_names();
    let rows = font_rows("Inter", "");
    let mut expected: Vec<String> = FAMILIES.iter().map(|f| f.name.to_owned()).collect();
    expected.extend([FONT_A.to_owned(), FONT_B.to_owned()]);
    assert_eq!(names(&rows), expected);
    assert_eq!(rows[0], FontRow::Header(LAMINA));
    assert_eq!(rows[7], FontRow::Header(COMPUTER));
    assert!(matches!(&rows[1], FontRow::Font { current: true, .. }));
    for name in [FONT_A, FONT_B] {
        assert!(font_preview(name, PREVIEW_PX).is_some(), "{name}");
    }
}

#[test]
fn fs_b2_a_busca_ignora_maiusculas_e_acentos() {
    let _fonts = fonts();
    installed_names();
    for query in ["gar", "GARAMOND", "garamônd"] {
        assert_eq!(
            names(&font_rows("Inter", query)),
            ["EB Garamond", "Cormorant Garamond"],
            "{query}"
        );
    }
    assert_eq!(
        font_rows("Inter", "xyz"),
        [FontRow::Note("Nenhuma fonte com “xyz”".to_owned())]
    );
}

#[test]
fn fs_b3_escolher_uma_fonte_do_computador() {
    let _fonts = fonts();
    let mut editor = editor_at(album(&["Inter"]), 0);
    editor.select_frame(Some(0));
    assert!(editor.change_text(TextChange::Font(FONT_A.to_owned())));
    assert_eq!(font_of(&editor), FONT_A);
    let label = editor.session().unwrap().undo_label().unwrap().to_owned();
    assert!(label.starts_with("Fonte"), "{label}");
    assert!(editor.run(Command::Undo));
    assert_eq!(font_of(&editor), "Inter");
}

#[test]
fn fs_b4_negrito_e_italico_seguem_os_arquivos_da_familia() {
    let _fonts = fonts();
    let mut editor = editor_at(album(&[FONT_A, FONT_B]), 0);
    editor.select_frame(Some(0));
    let a = editor.frame_form().unwrap().text.unwrap();
    assert!(!a.can_bold && !a.can_italic);
    editor.select_frame(Some(1));
    let b = editor.frame_form().unwrap().text.unwrap();
    assert!(b.can_bold && b.can_italic);
}

#[test]
fn fs_b6_fonte_que_nao_esta_neste_computador() {
    let _fonts = fonts();
    let dir = tempfile::tempdir().unwrap();
    let mut editor = opened(album(&["Lobster"]), dir.path());
    editor.select_frame(Some(0));
    let form = editor.frame_form().unwrap().text.unwrap();
    assert_eq!(form.font, "Lobster");
    assert_eq!(form.font_label, "Lobster (não está neste computador)");
    assert_eq!(
        font_rows("Lobster", "")[0],
        FontRow::Font {
            name: "Lobster".to_owned(),
            current: true,
            missing: true
        }
    );
    let again = opened(editor.session().unwrap().project().clone(), dir.path());
    let saved = &again.session().unwrap().project().spreads[0].frames[0].content;
    assert!(matches!(saved, FrameContent::Text(t) if t.font == "Lobster"));
}

#[test]
fn fs_b7_o_aviso_ao_abrir_o_album() {
    let _fonts = fonts();
    let dir = tempfile::tempdir().unwrap();
    let mut editor = opened(
        album(&["Lobster", "Pacifico", "Inter", "Lobster"]),
        dir.path(),
    );
    assert_eq!(
        editor.take_notice().as_deref(),
        Some(
            "Este álbum usa fontes que não estão neste computador: Lobster, Pacifico. Os textos \
             dessas fontes aparecem em Inter. Instale as fontes e abra o álbum de novo."
        )
    );
    assert_eq!(editor.take_notice(), None);
    let mut fine = opened(album(&["Inter", FONT_A]), dir.path());
    assert_eq!(fine.take_notice(), None);
}

#[test]
fn fs_b8_a_ia_usa_uma_fonte_do_computador() {
    let _fonts = fonts();
    let base = Base::with_photos(30, (600, 400));
    let (mut lab, _) = Lab::opened(&base, true);
    let added = lab.ok(
        "adicionar_texto",
        json!({ "lamina": 2, "pagina": "esquerda", "texto": "Ana", "fonte": "fonte a" }),
    );
    let quadro = usize::try_from(added.data["quadro"].as_u64().unwrap()).unwrap();
    let content = &lab.project().spreads[1].frames[quadro - 1].content;
    assert!(matches!(content, FrameContent::Text(t) if t.font == FONT_A));
    let refused = lab
        .call(
            "editar_texto",
            json!({ "lamina": 2, "quadro": quadro, "fonte": "Fonte" }),
        )
        .unwrap_err();
    assert!(
        matches!(&refused, ToolError::NoFont { name, known }
            if name == "Fonte" && known.starts_with("Inter, Montserrat") && known.contains(FONT_A)),
        "{refused}"
    );
}

#[test]
fn fs_b9_com_a_trava_fechada_a_escolha_volta_a_ser_a_de_hoje() {
    let _fonts = fonts();
    let _closed = Closed::new();
    installed_names();
    let rows = font_rows("Inter", "");
    assert!(!rows.contains(&FontRow::Header(COMPUTER)));
    assert_eq!(names(&rows).len(), 6);
    let base = Base::with_photos(30, (600, 400));
    let (mut lab, _) = Lab::opened(&base, true);
    let refused = lab
        .call(
            "adicionar_texto",
            json!({ "lamina": 2, "pagina": "esquerda", "texto": "Ana", "fonte": FONT_A }),
        )
        .unwrap_err();
    assert!(matches!(&refused, ToolError::NoFont { known, .. } if !known.contains(FONT_A)));
    // O desenho não é travado: o texto em "Fonte A" continua nela, sem aviso.
    let project = album(&[FONT_A]);
    assert!(missing_fonts(&project).is_empty());
    let mut editor = editor_at(project, 0);
    editor.select_frame(Some(0));
    assert_eq!(
        editor.frame_form().unwrap().text.unwrap().font_label,
        FONT_A
    );
}

#[test]
fn fs_b11_a_janela_nao_espera_a_leitura_das_fontes() {
    let (fonts, release) = TestFonts::gated();
    set_system_fonts_allowed(true);
    let rows = font_rows("Inter", "");
    assert!(rows.contains(&FontRow::Note(LOADING.to_owned())));
    let mut editor = editor_at(album(&["Inter"]), 0);
    editor.select_frame(Some(0));
    assert!(editor.change_text(TextChange::Font("Montserrat".to_owned())));
    assert_eq!(font_of(&editor), "Montserrat");
    release.send(()).unwrap();
    wait_names();
    assert!(names(&font_rows("Montserrat", "")).contains(&FONT_A.to_owned()));
    assert_eq!(fonts.reads().names, 1);
}

#[test]
fn fs_b12_busca_numa_lista_grande_e_rapido() {
    let _fonts = TestFonts::many(2000);
    set_system_fonts_allowed(true);
    assert_eq!(installed_names().len(), 2002);
    let mut best = Duration::MAX;
    for _ in 0..TRIES {
        let started = Instant::now();
        let rows = font_rows("Inter", "sans");
        best = best.min(started.elapsed());
        assert!(!rows.is_empty());
        if best < budget(5) {
            break;
        }
    }
    assert!(best < budget(5), "a busca levou {best:?}");
}

#[test]
fn fs_b13_quem_nao_usa_fontes_do_computador_nao_paga_nada() {
    let fonts = fonts();
    let dir = tempfile::tempdir().unwrap();
    let names: Vec<&str> = FAMILIES.iter().map(|f| f.name).collect();
    let mut editor = opened(album(&names), dir.path());
    assert_eq!(editor.take_notice(), None);
    editor.select_frame(Some(0));
    assert!(editor.change_text(TextChange::Font("Montserrat".to_owned())));
    assert!(editor.frame_form().unwrap().text.is_some());
    let _ = lamina_export::preflight(editor.session().unwrap().project(), &[0]);
    assert_eq!(fonts.reads().names, 0);
    assert_eq!(fonts.reads().files, 0);
}

#[test]
fn fs_b14_os_nomes_sao_lidos_uma_vez_e_so_quando_faz_falta() {
    let fonts = fonts();
    font_rows("Inter", "");
    wait_names();
    font_rows("Inter", "");
    let base = Base::with_photos(30, (600, 400));
    let (mut lab, _) = Lab::opened(&base, true);
    lab.ok(
        "adicionar_texto",
        json!({ "lamina": 2, "pagina": "esquerda", "texto": "Ana", "fonte": FONT_B }),
    );
    let dir = tempfile::tempdir().unwrap();
    opened(album(&[FONT_A]), dir.path());
    assert_eq!(fonts.reads().names, 1);
    drop(fonts);
    // Abrir primeiro um álbum com "Fonte A", sem abrir a lista, também lê os nomes uma vez.
    let fonts = self::fonts();
    opened(album(&[FONT_A]), dir.path());
    assert_eq!(fonts.reads().names, 1);
}

#[test]
fn fs_b16_previas_so_das_linhas_que_aparecem() {
    let _fonts = TestFonts::many(2000);
    let all = installed_names();
    let mut previews = FontPreviews::default();
    for window in all.chunks(12) {
        let wanted = previews.wanted(window);
        assert!(wanted.len() <= 12);
        for name in wanted {
            let image = preview(&name);
            previews.put(name, image);
        }
    }
    // Voltar a uma linha já vista não pede de novo.
    assert!(previews.wanted(&all[..12]).is_empty());
    assert_eq!(previews.asked(), all.len());
    assert!(previews.get(FONT_A).is_some());
    previews.clear();
    assert_eq!(previews.asked(), 0);
    assert!(previews.get(FONT_A).is_none());
}
