//! Velocidade do painel de fotos (PF-B14 e PF-B15; AGENTS.md, regra 13): as contas da grade com
//! 1.000 fotos, sem janela e sem ler o disco.

use std::time::{Duration, Instant};

use lamina_core::PhotoId;
use lamina_core::testing::{TRIES, budget};

use super::pf::{editor_with, photos};
use crate::motion::photo_rows;
use crate::photo_grid::Grid;
use crate::{Command, Filter, ThumbList, ThumbPlan, photo_list};

#[test]
fn pf_b14_as_linhas_de_1000_fotos_rapido() {
    let mut editor = editor_with(photos(1000));
    assert!(editor.run(Command::SelectAllPhotos));
    let mut best = Duration::MAX;
    for _ in 0..TRIES {
        let mut plan = ThumbPlan::default();
        let started = Instant::now();
        let project = editor.session().unwrap().project();
        let list = photo_list(project, Filter::All);
        let rows = photo_rows(&list, editor.selection(), Filter::All);
        let grid = Grid::new(rows.len(), 1200.0, 96);
        let visible = grid.visible(0.0, 4.0 * grid.row_height());
        let ids: Vec<PhotoId> = grid
            .thumb_order(visible)
            .into_iter()
            .map(|i| rows[i].0)
            .collect();
        let orders = plan.want(ThumbList::Browser, &ids);
        best = best.min(started.elapsed());
        assert!(rows.iter().all(|(_, r)| r.picked));
        assert_eq!(orders.load.len(), 8 * 11);
        if best < budget(10) {
            break;
        }
    }
    assert!(best < budget(10), "as linhas levaram {best:?}");
}

#[test]
fn pf_b15_rolar_rapido() {
    let shown = editor_with(photos(1000)).shown_photos();
    let grid = Grid::new(shown.len(), 1200.0, 96);
    let height = 4.0 * grid.row_height();
    let deepest = grid.max_scroll(height);
    let mut best = Duration::MAX;
    for _ in 0..TRIES {
        let mut plan = ThumbPlan::default();
        let mut last = Vec::new();
        let started = Instant::now();
        for step in 0..=200 {
            let scroll = deepest * step as f32 / 200.0;
            let visible = grid.visible(scroll, height);
            last = grid
                .thumb_order(visible)
                .into_iter()
                .map(|i| shown[i])
                .collect();
            plan.want(ThumbList::Browser, &last);
        }
        best = best.min(started.elapsed());
        // No fim, só as de visível ± margem continuam pedidas.
        assert_eq!(plan.pending(), last.len());
        if best < budget(20) {
            break;
        }
    }
    assert!(best < budget(20), "os 200 passos levaram {best:?}");
}
