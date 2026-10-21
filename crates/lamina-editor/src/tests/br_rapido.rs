//! Velocidade de "Montar o álbum…" (BR-B16; AGENTS.md, regra 13): 200 fotos não usadas em 50
//! lâminas novas, a conta e a edição.

use std::time::{Duration, Instant};

use lamina_core::testing::{HEAVY_TRIES, budget};
use lamina_layout::Reuse;

use super::tl_common::{H, V, built, editor_at, spec};
use crate::Command;

#[test]
fn br_b16_montar_rapido() {
    let photos: Vec<(u32, u32)> = (0..40).map(|i| if i % 4 == 3 { V } else { H }).collect();
    let mut project = built(spec(30.0, 30.0), &photos, 10);
    for i in 0..200 {
        let (w, h) = if i % 4 == 3 { V } else { H };
        project.add_photo(format!("g{i:03}.jpg"), w, h);
    }
    let mut editor = editor_at(project, 0);
    let mut best = Duration::MAX;
    for _ in 0..HEAVY_TRIES {
        let started = Instant::now();
        editor.build_album(Some(50), Reuse::Medium).unwrap();
        best = best.min(started.elapsed());
        assert_eq!(editor.session().unwrap().project().spreads.len(), 60);
        assert!(editor.run(Command::Undo));
        if best < budget(2000) {
            break;
        }
    }
    assert!(best < budget(2000), "montar levou {best:?}");
}
