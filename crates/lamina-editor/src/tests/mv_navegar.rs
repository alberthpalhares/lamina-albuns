//! Ir para outra lâmina (spec MV, regra 9 e seção 4): MV-B7 (desfazer que muda de lâmina), MV-B8,
//! MV-B9, MV-B11 e MV-B13 (a parte de navegar).

use std::time::Duration;

use lamina_core::Project;

use super::tl_common::{H, V, built, editor_at, spec};
use crate::Command;
use crate::motion::{Navigator, Place, Transition};

fn album(spreads: usize) -> Project {
    let photos: Vec<(u32, u32)> = (0..spreads * 4)
        .map(|i| if i % 4 == 3 { V } else { H })
        .collect();
    built(spec(30.0, 30.0), &photos, spreads)
}

fn ms(n: u64) -> Duration {
    Duration::from_millis(n)
}

fn here(editor: &crate::Editor) -> (Place, usize) {
    let s = editor.session().unwrap();
    ((1, s.project().spreads[s.current()].id), s.current())
}

#[test]
fn mv_b8_navegar_desliza_na_direcao_certa() {
    let mut editor = editor_at(album(8), 4);
    let mut nav = Navigator::default();
    let (p, i) = here(&editor);
    assert_eq!(
        nav.arrive(p, i, ms(0), false),
        Transition::Stay,
        "primeira exibição"
    );
    assert!(editor.run(Command::NextSpread));
    let (p, i) = here(&editor);
    assert_eq!(nav.arrive(p, i, ms(1_000), false), Transition::Forward);
    assert!(editor.run(Command::PreviousSpread));
    let (p, i) = here(&editor);
    assert_eq!(nav.arrive(p, i, ms(2_000), false), Transition::Backward);
    // Clique na lâmina 1 da faixa.
    assert!(editor.go_to_spread(0));
    let (p, i) = here(&editor);
    assert_eq!(nav.arrive(p, i, ms(3_000), false), Transition::Backward);
    // Uma edição na mesma lâmina não é navegar.
    assert!(editor.run(Command::NextLayout));
    let (p, i) = here(&editor);
    assert_eq!(nav.arrive(p, i, ms(4_000), false), Transition::Stay);
}

#[test]
fn mv_b9_navegacao_seguida_nao_anima() {
    let mut editor = editor_at(album(8), 2);
    let mut nav = Navigator::default();
    let (p, i) = here(&editor);
    nav.arrive(p, i, ms(0), false);
    assert!(editor.run(Command::NextSpread));
    let (p, i) = here(&editor);
    assert_eq!(nav.arrive(p, i, ms(1_000), false), Transition::Forward);
    // Page Down de novo 50 ms depois: sem animação.
    assert!(editor.run(Command::NextSpread));
    let (p, i) = here(&editor);
    assert_eq!(nav.arrive(p, i, ms(1_050), false), Transition::Cut);
    // Passados os 200 ms, volta a deslizar.
    assert!(editor.run(Command::NextSpread));
    let (p, i) = here(&editor);
    assert_eq!(nav.arrive(p, i, ms(1_400), false), Transition::Forward);
}

#[test]
fn mv_b11_outro_album_nao_desliza() {
    let editor = editor_at(album(4), 1);
    let mut nav = Navigator::default();
    let ((_, id), i) = here(&editor);
    nav.arrive((1, id), i, ms(0), false);
    // O álbum B, com os mesmos ids de lâmina, em outra lâmina.
    assert_eq!(
        nav.arrive((2, id), i + 1, ms(5_000), false),
        Transition::Cut
    );
}

#[test]
fn mv_b13_reduzir_movimento_navega_sem_animacao() {
    let mut editor = editor_at(album(8), 2);
    let mut nav = Navigator::default();
    let (p, i) = here(&editor);
    nav.arrive(p, i, ms(0), true);
    assert!(editor.run(Command::NextSpread));
    let (p, i) = here(&editor);
    assert_eq!(nav.arrive(p, i, ms(1_000), true), Transition::Cut);
}

#[test]
fn mv_b7_desfazer_que_muda_de_lamina_desliza_como_navegar() {
    let mut editor = editor_at(album(4), 1);
    let mut nav = Navigator::default();
    assert!(editor.run(Command::NextLayout));
    assert!(editor.run(Command::NextSpread));
    let (p, i) = here(&editor);
    nav.arrive(p, i, ms(0), false);
    assert!(editor.run(Command::Undo));
    let (p, i) = here(&editor);
    assert_eq!(i, 1);
    assert_eq!(nav.arrive(p, i, ms(1_000), false), Transition::Backward);
}
