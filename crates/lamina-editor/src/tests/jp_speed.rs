//! Abrir e navegar rápido (JP-B13; AGENTS.md, regra 13).

use std::time::{Duration, Instant};

use lamina_core::testing::budget;
use lamina_layout::Reuse;

use super::common::Base;
use crate::{NewAlbum, ScreenImages, Session, SpecFields, create_album};

#[test]
fn jp_b13_abrir_e_navegar_rapido() {
    let base = Base::with_photos(40, (3000, 2000));
    let cache = base.cache();
    let new = NewAlbum {
        folder: base.path("fotos"),
        name: Some("casamento".into()),
        spec: SpecFields::default().to_spec().unwrap(),
        spreads: Some(10),
        reuse: Reuse::Medium,
        preferences: lamina_layout::Preferences::default(),
    };
    // Monta uma vez: deixa miniaturas e imagens de tela no cache.
    let (built, _) = create_album(&new, &cache, &|_| {}).unwrap();
    let file = base.save(built.project, "album/casamento.lamina");

    let mut open = Duration::MAX;
    let mut step = Duration::MAX;
    for _ in 0..3 {
        let started = Instant::now();
        let mut session = Session::open(&file).unwrap();
        let mut screens = ScreenImages::new();
        screens.keep_around(session.project(), 0, &cache);
        open = open.min(started.elapsed());
        assert!(!screens.is_empty());

        for next in 1..session.spread_count() {
            let started = Instant::now();
            assert!(session.go_to(next));
            screens.keep_around(session.project(), next, &cache);
            step = step.min(started.elapsed());
        }
    }
    assert!(open < budget(500), "abrir levou {open:?}");
    assert!(step < budget(100), "trocar de lâmina levou {step:?}");
}
