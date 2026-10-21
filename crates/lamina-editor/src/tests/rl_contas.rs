//! As contas do rolo de lâminas (spec RL, regras 2, 11 a 19, 21, 22, 24 e 26): largura, ir para
//! outra lâmina, a roda, a pílula, o selo, a barra, a alça, o acerto, a linha do tempo, os modos
//! e "Reduzir movimento" (RL-B10 a RL-B19, RL-B21 e RL-B25).

use std::time::Duration;

use lamina_core::{AlbumEdge, Edges, Project};

use super::tl_common::{H, V, built, editor_at, spec};
use crate::motion::{
    SPREAD_BAR_COMMANDS, Durations, Navigator, Pill, Roll, RollHit, RollMove, RollScroll, TIMELINE,
    Transition, edge_speed, pill, reorder, roll_gap, roll_hit, roll_move, roll_shown, roll_width,
    spread_badge, spread_bar, timeline_rows, timeline_run, timeline_scroll, visible_range,
};
use crate::{Command, DropPlace, Editor};

fn album(spreads: usize) -> Project {
    let photos: Vec<(u32, u32)> = (0..spreads * 4)
        .map(|i| if i % 4 == 3 { V } else { H })
        .collect();
    built(spec(30.0, 30.0), &photos, spreads)
}

fn last_label(editor: &Editor) -> String {
    let list = editor.session().unwrap().history_list();
    list.steps
        .last()
        .map(|s| s.label.clone())
        .unwrap_or_default()
}

/// O rolo do RL-B17: lâmina de 1.000 × 500 px numa área de 1.400 px.
fn roll(count: usize) -> Roll {
    Roll {
        area: (1400.0, 600.0),
        width: 1000.0,
        height: 500.0,
        count,
    }
}

#[test]
fn rl_b10_vizinha_desliza_e_cliques_rapidos_trocam_sem_animar() {
    let project = album(10);
    let place = |n: usize| (1, project.spreads[n - 1].id);
    let mut nav = Navigator::default();
    nav.arrive(place(5), 4, Duration::ZERO, false);
    // Clique na 6: uma posição do que se vê, desliza.
    let t = nav.arrive(place(6), 5, Duration::from_millis(1000), false);
    assert_eq!(roll_move(4.0, 5, t), RollMove::Slide);
    // 50 ms depois, a 7: navegação seguida, sem animar (MV-B9).
    let t = nav.arrive(place(7), 6, Duration::from_millis(1050), false);
    assert_eq!(roll_move(5.0, 6, t), RollMove::Cut);
}

#[test]
fn rl_b11_distancia_conta_a_partir_do_que_se_ve_no_centro() {
    // Atual 5, rolado à mão até a 12 no centro: clicar na 13 desliza.
    assert_eq!(roll_move(11.0, 12, Transition::Forward), RollMove::Slide);
    // Page Down vai à 14, a uma posição da 13 no centro: também desliza.
    assert_eq!(roll_move(12.0, 13, Transition::Forward), RollMove::Slide);
    // Home a partir dali: 13 posições, esmaece.
    assert_eq!(roll_move(13.0, 0, Transition::Backward), RollMove::Fade(-1));
    assert_eq!(roll_move(4.0, 7, Transition::Forward), RollMove::Fade(1));
    assert_eq!(roll_move(4.0, 4, Transition::Stay), RollMove::Stay);
}

#[test]
fn rl_b12_roda_e_touchpad_rolam_de_lado_sem_trocar_a_lamina() {
    let mut editor = editor_at(album(10), 4);
    let mut scroll = RollScroll::default();
    scroll.follow(4);
    let step = 1024.0;
    for _ in 0..3 {
        scroll.wheel(-120.0, step, 10);
    }
    let after_wheel = scroll.target;
    assert!(after_wheel > 4.0, "a roda para baixo avança");
    scroll.wheel(-200.0, step, 10);
    assert!(scroll.target > after_wheel, "o touchpad soma");
    assert!(!scroll.follow, "rolar à mão para de seguir a lâmina atual");
    assert_eq!(
        editor.session().unwrap().current(),
        4,
        "a atual continua a 5"
    );
    scroll.wheel(-1_000_000.0, step, 10);
    assert!((scroll.target - 10.0).abs() < 1e-9, "para no + Lâmina nova");
    // Navegar volta a seguir.
    assert!(editor.run(Command::NextSpread));
    scroll.follow(editor.session().unwrap().current());
    assert!(scroll.follow);
    assert!((scroll.target - 5.0).abs() < 1e-9);
}

#[test]
fn rl_b13_pilula_volta_o_rolo_sem_trocar_nada() {
    let editor = editor_at(album(12), 4);
    let r = roll(12);
    let visible = visible_range(&r, 8.0).unwrap();
    assert_eq!(
        pill(4, &visible),
        Some(Pill {
            left: true,
            label: "← Lâmina 5".into()
        })
    );
    let mut scroll = RollScroll {
        target: 8.0,
        follow: false,
    };
    scroll.follow(4);
    assert!((scroll.target - 4.0).abs() < 1e-9);
    let s = editor.session().unwrap();
    assert_eq!(s.current(), 4);
    assert!(s.history_list().steps.is_empty(), "nenhuma edição");
    assert_eq!(pill(4, &visible_range(&r, 4.0).unwrap()), None);
    assert_eq!(
        pill(4, &visible_range(&r, 0.0).unwrap()).map(|p| p.label),
        Some("Lâmina 5 →".into())
    );
}

#[test]
fn rl_b14_selo_com_a_lamina_e_as_paginas() {
    let full = Edges::default();
    assert_eq!(spread_badge(full, 0, 10), "Lâmina 1 · páginas 1 e 2");
    assert_eq!(spread_badge(full, 4, 10), "Lâmina 5 · páginas 9 e 10");
    assert_eq!(spread_badge(full, 9, 10), "Lâmina 10 · páginas 19 e 20");
    let single = Edges::new(AlbumEdge::SinglePage, AlbumEdge::SinglePage);
    assert_eq!(spread_badge(single, 0, 10), "Lâmina 1 · página 1");
    assert_eq!(spread_badge(single, 4, 10), "Lâmina 5 · páginas 8 e 9");
    assert_eq!(spread_badge(single, 9, 10), "Lâmina 10 · página 18");
}

#[test]
fn rl_b15_barra_da_vizinha_traz_ela_e_roda_o_comando() {
    let mut editor = editor_at(album(10), 4);
    let bar = spread_bar(&editor, 5).unwrap();
    assert_eq!(bar.badge, "Lâmina 6 · páginas 11 e 12");
    let keys: Vec<&str> = bar.buttons.iter().map(|b| b.key.as_str()).collect();
    assert_eq!(keys, vec!["O", "L", "E", "T"]);
    assert_eq!(bar.buttons[2].tip, "Espelhar lâmina (E)");
    for b in &bar.buttons {
        assert_eq!(
            b.enabled,
            editor.available(b.command),
            "a disponibilidade do menu"
        );
    }
    // Espelhar na barra da 6: ela vira a atual e a edição cita a lâmina 6.
    assert!(editor.go_to_spread(5));
    assert!(editor.run(SPREAD_BAR_COMMANDS[2]));
    assert_eq!(editor.session().unwrap().current(), 5);
    assert_eq!(last_label(&editor), "Espelhar lâmina · lâmina 6");
    // No modo organizar, a barra não aparece.
    assert!(editor.run(Command::Organize));
    assert!(spread_bar(&editor, 5).is_none());
}

#[test]
fn rl_b16_alca_reordena_pelo_vao_com_rolagem_nas_bordas() {
    let mut editor = editor_at(album(8), 1);
    let r = roll(8);
    // O ponteiro no meio do vão entre a 5 e a 6, com a 2 no centro.
    let x = r.left(5, 1.0) - 12.0;
    let gap = roll_gap(&r, 1.0, x);
    assert_eq!(gap, 5);
    let plan = reorder(1, gap, 8);
    assert_eq!(plan.to, 4, "a posição 5");
    assert_eq!(&plan.shift[2..=4], &[-1, -1, -1]);
    assert!(editor.move_current_to(plan.to));
    assert_eq!(last_label(&editor), "Mover lâmina · 2 → 5");
    assert_eq!(editor.session().unwrap().current(), 4);
    let w = r.area.0;
    assert!((edge_speed(w, w) - 2.0).abs() < 1e-9);
    assert!((edge_speed(w - 24.0, w) - 1.0).abs() < 1e-9);
    assert!(edge_speed(w - 48.0, w).abs() < 1e-9);
    assert!(
        (edge_speed(0.0, w) + 2.0).abs() < 1e-9,
        "na borda esquerda, para trás"
    );
}

#[test]
fn rl_b17_acerto_no_rolo() {
    let r = roll(8);
    let pos = 4.0;
    let left6 = r.left(5, pos);
    let hit = roll_hit(&r, pos, (left6 + 100.0, r.top() + 250.0));
    assert_eq!(
        hit,
        Some(RollHit::Spread {
            index: 5,
            fx: 0.1,
            fy: 0.25
        })
    );
    let middle = left6 - 12.0;
    assert_eq!(
        roll_hit(&r, pos, (middle, r.top() + 10.0)),
        Some(RollHit::Gap(5))
    );
    // Seis lâminas, rolado até o fim: à direita do "+ Lâmina nova", nada.
    let six = roll(6);
    let new = six.left(6, 6.0);
    assert_eq!(
        roll_hit(&six, 6.0, (new + 10.0, six.top() + 10.0)),
        Some(RollHit::New)
    );
    assert_eq!(roll_hit(&six, 6.0, (new + 1100.0, six.top() + 10.0)), None);
    // Soltar 4 fotos no "+ Lâmina nova" cria a lâmina 7 com elas (IM-B7).
    let mut editor = editor_at(album(6), 0);
    let mut project = editor.session().unwrap().project().clone();
    let photos: Vec<_> = (0..4)
        .map(|i| project.add_photo(format!("n{i}.jpg"), 3000, 2000))
        .collect();
    editor.adopt(project);
    assert!(editor.drop_photos(DropPlace::NewSpread(6), &photos));
    let s = editor.session().unwrap();
    assert_eq!(s.spread_count(), 7);
    let mut used = crate::edits::spread_photos(s.project(), 6);
    used.sort();
    assert_eq!(used, photos);
}

#[test]
fn rl_b18_linha_do_tempo_destaca_centraliza_e_leva_a_lamina() {
    let mut editor = editor_at(album(10), 4);
    let rows = timeline_rows(editor.session().unwrap().project());
    let run = timeline_run(&rows, 4, TIMELINE);
    assert_eq!(run, (838.0, 190.0));
    let content = timeline_run(&rows, 10, TIMELINE).0 + TIMELINE.padding;
    let view = 600.0;
    let offset = timeline_scroll(run, view, content);
    assert!((offset + view / 2.0 - (run.0 + run.1 / 2.0)).abs() < 1e-9);
    // Clicar numa foto da lâmina 7: vai a ela, deslizando (2 posições).
    let spread = rows.iter().find(|(_, s)| *s == 6).map(|(_, s)| *s).unwrap();
    let mut nav = Navigator::default();
    let s = editor.session().unwrap();
    nav.arrive((1, s.project().spreads[4].id), 4, Duration::ZERO, false);
    assert!(editor.go_to_spread(spread));
    let s = editor.session().unwrap();
    let t = nav.arrive(
        (1, s.project().spreads[6].id),
        6,
        Duration::from_secs(1),
        false,
    );
    assert_eq!(roll_move(4.0, 6, t), RollMove::Slide);
}

#[test]
fn rl_b19_modo_organizar_e_recorte_escondem_o_rolo() {
    let mut editor = editor_at(album(6), 2);
    assert!(roll_shown(&editor));
    assert!(editor.run(Command::Organize));
    assert!(!roll_shown(&editor), "no modo organizar");
    assert!(editor.run(Command::Organize));
    assert!(roll_shown(&editor), "ao sair, volta");
    assert!(editor.enter_crop(0));
    assert!(!roll_shown(&editor), "no recorte");
}

#[test]
fn rl_b21_reduzir_movimento_zera_o_rolo_e_mantem_as_bordas() {
    let d = Durations::new(true);
    assert_eq!((d.navigate, d.scroll, d.fade, d.reorder), (0, 0, 0, 0));
    let project = album(3);
    let place = |n: usize| (1, project.spreads[n - 1].id);
    let mut nav = Navigator::default();
    nav.arrive(place(1), 0, Duration::ZERO, true);
    let t = nav.arrive(place(2), 1, Duration::from_secs(1), true);
    assert_eq!(roll_move(0.0, 1, t), RollMove::Cut);
    assert!((edge_speed(1400.0, 1400.0) - 2.0).abs() < 1e-9);
}

#[test]
fn rl_b25_largura_da_lamina_com_as_vizinhas() {
    let wide = roll_width((1400.0, 900.0), 0.0, 0.5);
    assert!((wide.width - 1128.0).abs() < 1e-9);
    assert!(wide.neighbors);
    let low = roll_width((1400.0, 500.0), 0.0, 0.5);
    assert!((low.width - 904.0).abs() < 1e-9);
    let narrow = roll_width((600.0, 900.0), 0.0, 0.5);
    assert!((narrow.width - 552.0).abs() < 1e-9);
    assert!(!narrow.neighbors);
}
