use lamina_core::{ActivePages, AlbumSpec, Dpi, Mm};

use super::*;
use crate::layouts::{LayoutOptions, layout_options};

const H: f64 = 1.5;
const V: f64 = 2.0 / 3.0;

fn lists(aspects: &[f64]) -> Vec<LayoutCandidate> {
    let spec = AlbumSpec::new(Mm::from_cm(30.0), Mm::from_cm(30.0), Dpi::new(300).unwrap())
        .unwrap()
        .with_safe_margin(Mm(10.0))
        .unwrap();
    let both = ActivePages {
        left: true,
        right: true,
    };
    layout_options(&spec, both, aspects, &LayoutOptions::default())
        .unwrap()
        .in_order
}

#[test]
fn desenho_tem_orientacoes_familia_e_respiro() {
    let list = lists(&[H, H, V]);
    let key = design_key(&list[0], &[H, H, V]);
    assert!(key.starts_with("HHV|"), "{key}");
    assert!(
        key.ends_with("c") || key.ends_with('s') || key.ends_with('e'),
        "{key}"
    );
    let keys: std::collections::HashSet<_> =
        list.iter().map(|c| design_key(c, &[H, H, V])).collect();
    assert!(keys.len() * 10 >= list.len() * 9, "quase todos distintos");
}

#[test]
fn favorito_sobe_e_rejeitado_some() {
    let aspects = [H, H, V];
    let list = lists(&aspects);
    let (fav, rej) = (
        design_key(&list[6], &aspects),
        design_key(&list[0], &aspects),
    );
    let mut prefs = Preferences::default();
    prefs.toggle_favorite(&fav);
    prefs.reject(&rej);
    let out = apply_preferences(list.clone(), &aspects, &prefs);
    assert_eq!(design_key(&out.list[0], &aspects), fav);
    assert!(out.favorite[0] && !out.favorite[1]);
    assert!(out.list.iter().all(|c| design_key(c, &aspects) != rej));
    assert!(!out.all_rejected);
    // O resto segue a ordem da variedade.
    assert_eq!(out.list[1], list[1]);
}

#[test]
fn todos_rejeitados_volta_a_lista_inteira() {
    let aspects = [H];
    let list = lists(&aspects);
    let mut prefs = Preferences::default();
    for c in &list {
        prefs.reject(&design_key(c, &aspects));
    }
    let out = apply_preferences(list.clone(), &aspects, &prefs);
    assert!(out.all_rejected);
    assert_eq!(out.list, list);
}

#[test]
fn favoritar_tira_dos_rejeitados_e_desmarca() {
    let mut prefs = Preferences::default();
    prefs.reject("x");
    prefs.toggle_favorite("x");
    assert!(prefs.favorites.contains("x") && !prefs.rejected.contains("x"));
    prefs.toggle_favorite("x");
    assert!(prefs.is_empty());
}
