//! Cenários PF da seleção, das abas, do esmaecido e do contador, e a rolagem do laço.

use std::collections::HashSet;

use lamina_core::PhotoId;

use super::pf::{editor_with, photos, use_photos};
use super::tl_common::{H, built, editor_at, spec};
use crate::commands::Mode;
use crate::motion::{photo_ghost, photo_rows, row_ops};
use crate::photo_grid::{Grid, LASSO_SPEED, lasso_scroll};
use crate::{Command, DropPlace, Editor, Filter, Shortcut, photo_list, photos_title};

/// 10 fotos, 4 usadas (a 3ª em dois quadros), e as fotos de números `picked` selecionadas.
fn ten(picked: &[usize]) -> (Editor, Vec<PhotoId>) {
    let mut project = photos(10);
    let ids: Vec<PhotoId> = project.photos.iter().map(|p| p.id).collect();
    use_photos(&mut project, &[ids[0], ids[1], ids[2], ids[2], ids[3]]);
    let mut editor = editor_at(project, 0);
    for (k, n) in picked.iter().enumerate() {
        editor.click_photo(&ids, n - 1, k > 0, false);
    }
    (editor, ids)
}

fn rows(editor: &Editor, filter: Filter) -> Vec<(PhotoId, crate::motion::PhotoRow)> {
    let project = editor.session().unwrap().project();
    photo_rows(&photo_list(project, filter), editor.selection(), filter)
}

#[test]
fn pf_b8_trocar_a_aba_refaz_as_linhas_e_mantem_a_selecao() {
    // 5 selecionadas, 3 delas usadas.
    let (mut editor, ids) = ten(&[1, 2, 3, 6, 8]);
    editor.set_photo_filter(Filter::Unused);
    assert_eq!(editor.shown_photos(), [4, 5, 6, 7, 8, 9].map(|i| ids[i]));
    assert_eq!(editor.selection().len(), 5);
    let title = photos_title(editor.session().unwrap(), editor.selection().len());
    assert!(title.ends_with("· 5 selecionadas"), "{title}");
    // Arrastar uma das 2 selecionadas que aparecem leva as 5, com o selo.
    assert_eq!(editor.drag_photos(ids[5]).len(), 5);
    let project = editor.session().unwrap().project();
    let ghost = photo_ghost(project, ids[5], editor.selection()).unwrap();
    assert_eq!(ghost.badge, Some(5));
}

#[test]
fn pf_b9_usadas_esmaecidas_e_o_contador() {
    let (mut editor, ids) = ten(&[1, 5, 9]);
    let all = rows(&editor, Filter::All);
    let dim: Vec<PhotoId> = all
        .iter()
        .filter(|(_, r)| r.dim)
        .map(|(id, _)| *id)
        .collect();
    assert_eq!(dim, ids[..4]);
    let uses: Vec<usize> = all.iter().map(|(_, r)| r.item.uses).collect();
    assert_eq!(uses, [1, 1, 2, 1, 0, 0, 0, 0, 0, 0]);
    assert!(rows(&editor, Filter::Used).iter().all(|(_, r)| !r.dim));
    let session = editor.session().unwrap();
    assert_eq!(
        photos_title(session, editor.selection().len()),
        "Fotografias · 4 de 10 usadas · 3 selecionadas"
    );
    assert!(editor.run(Command::Deselect));
    let session = editor.session().unwrap();
    assert_eq!(photos_title(session, 0), "Fotografias · 4 de 10 usadas");
    assert_eq!(
        photos_title(session, 1),
        "Fotografias · 4 de 10 usadas · 1 selecionada"
    );
}

#[test]
fn pf_b10_colocar_uma_foto_atualiza_so_a_linha_dela() {
    let mut project = photos(1000);
    project.add_spread();
    let mut editor = editor_at(project, 0);
    let shown = editor.shown_photos();
    let grid = Grid::new(1000, 1200.0, 96);
    let (all, unused) = (rows(&editor, Filter::All), rows(&editor, Filter::Unused));
    assert!(editor.new_photo_frame(None));
    let frame = editor.selected_frame().unwrap();
    assert!(editor.drop_photo(frame, shown[499]));
    let placed = rows(&editor, Filter::All);
    let unused_after = rows(&editor, Filter::Unused);
    assert_eq!(grid.touched_rows(&row_ops(&all, &placed)), [45]);
    assert_eq!(grid.row_range(45), 495..506, "fotos 496 a 506");
    // Clicar muda só as linhas das fotos que mudaram de seleção.
    editor.click_photo(&shown, 19, false, false);
    let clicked = rows(&editor, Filter::All);
    assert_eq!(grid.touched_rows(&row_ops(&placed, &clicked)), [1]);
    // Na aba Não usadas, a foto sai da lista: da linha dela até o fim.
    let ops = row_ops(&unused, &unused_after);
    assert_eq!(grid.touched_rows(&ops), (45..91).collect::<Vec<_>>());
}

#[test]
fn pf_b11_arrastar_a_selecao_para_a_lamina_igual_a_hoje() {
    let mut project = built(spec(30.0, 30.0), &[H; 24], 6);
    for i in 0..30 {
        project.add_photo(format!("g{i:02}.jpg"), H.0, H.1);
    }
    let mut editor = editor_at(project, 0);
    let shown = editor.shown_photos();
    // 6 fotos não usadas em 3 linhas da grade de 11 colunas.
    for (k, i) in [30, 31, 40, 41, 50, 51].into_iter().enumerate() {
        editor.click_photo(&shown, i, k > 0, false);
    }
    let photos = editor.drag_photos(shown[40]);
    assert_eq!(photos.len(), 6);
    assert!(editor.drop_photos(DropPlace::NewSpread(6), &photos));
    let session = editor.session().unwrap();
    assert_eq!(session.project().spreads.len(), 7);
    let label = session.history_list().steps.last().unwrap().label.clone();
    assert!(
        label.starts_with("Nova lâmina com 6 fotos · lâmina 7"),
        "{label}"
    );
    // Uma foto fora da seleção leva só ela.
    editor.click_photo(&shown, 32, false, false);
    editor.click_photo(&shown, 33, true, false);
    assert_eq!(editor.drag_photos(shown[45]), [shown[45]]);
}

#[test]
fn pf_b13_selecionar_todas_as_mostradas() {
    let mut project = built(spec(30.0, 30.0), &[H; 180], 45);
    for i in 0..120 {
        project.add_photo(format!("g{i:03}.jpg"), H.0, H.1);
    }
    let mut editor = editor_at(project, 0);
    editor.set_photo_filter(Filter::Unused);
    let ctrl_a = Shortcut::ctrl('a');
    assert_eq!(
        Command::from_shortcut_in(ctrl_a, Mode::Normal),
        Some(Command::SelectAllPhotos)
    );
    assert!(editor.run(Command::SelectAllPhotos));
    assert_eq!(editor.selection().len(), 120);
    assert_eq!(editor.selection(), editor.shown_photos());
    // No modo organizar, Ctrl+A seleciona os quadros.
    assert!(editor.run(Command::Organize));
    assert_eq!(
        Command::from_shortcut_in(ctrl_a, Mode::Organize),
        Some(Command::SelectAllFrames)
    );
    assert!(!editor.available(Command::SelectAllPhotos));
    assert!(editor.run(Command::SelectAllFrames));
    assert!(!editor.selected_frames().is_empty());
    for mode in [Mode::Normal, Mode::Organize] {
        let mut seen = HashSet::new();
        for c in Command::ALL.into_iter().filter(|c| c.in_mode(mode)) {
            if let Some(s) = c.shortcut() {
                assert!(seen.insert(s), "{s:?} repetido no modo {mode:?}");
            }
        }
    }
}

#[test]
fn pf_b16_o_laco_rola_a_grade_perto_da_borda() {
    let height = 300.0;
    assert_eq!(lasso_scroll(150.0, height), 0.0);
    let (near, nearer) = (lasso_scroll(290.0, height), lasso_scroll(299.0, height));
    assert!(near > 0.0, "a 290 rola para baixo");
    assert!(nearer > near, "a 299 mais rápido que a 290");
    assert_eq!(lasso_scroll(320.0, height), LASSO_SPEED, "fora: o máximo");
    assert!(lasso_scroll(5.0, height) < 0.0, "perto do topo, para cima");
}

#[test]
fn pf_selecao_fica_na_ordem_do_painel() {
    let mut editor = editor_with(photos(30));
    let shown = editor.shown_photos();
    editor.lasso_photos(&[shown[20], shown[3]], false);
    editor.lasso_photos(&[shown[10]], true);
    assert_eq!(editor.selection(), [shown[3], shown[10], shown[20]]);
}
