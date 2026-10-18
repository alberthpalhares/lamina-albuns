//! PR-B10 (spec `presets`): `lamina presets`, `lamina build --preset` e preset que não existe.

#![allow(clippy::unwrap_used)]

mod common;

use common::{Base, text};
use lamina_core::Project;

#[test]
fn pr_b10_presets_pela_linha_de_comando() {
    let base = Base::with_photos(6, (300, 200));
    let list = base.lamina(&["presets"]);
    assert!(list.status.success(), "{}", text(&list.stderr));
    let out = text(&list.stdout);
    for name in [
        "Quadrado 20 × 20",
        "Quadrado 25 × 25",
        "Quadrado 30 × 30",
        "Paisagem 30 × 20",
        "Paisagem 40 × 30",
        "Retrato 20 × 30",
    ] {
        assert!(out.contains(name), "{name}: {out}");
    }

    let cache = base.path("cache");
    let build = base.lamina(&[
        "build",
        "fotos",
        "--saida",
        "saida",
        "--nome",
        "album",
        "--laminas",
        "2",
        "--dpi",
        "72",
        "--preset",
        "Retrato 20 × 30",
        "--cache",
        cache.to_str().unwrap(),
    ]);
    assert!(build.status.success(), "{}", text(&build.stderr));
    let json = std::fs::read_to_string(base.path("saida/album.lamina")).unwrap();
    let project = Project::from_json(&json).unwrap();
    assert!((project.spec.page_width().get() - 200.0).abs() < 1e-9);
    assert!((project.spec.page_height().get() - 300.0).abs() < 1e-9);
    assert_eq!(
        project.spec.dpi().get(),
        72,
        "a opção digitada vale por cima do preset"
    );
    assert_eq!(project.page_limits.min, Some(20));

    let missing = base.lamina(&["build", "fotos", "--saida", "x", "--preset", "Não existe"]);
    assert_eq!(missing.status.code(), Some(1));
    assert_eq!(
        text(&missing.stderr).trim(),
        "Erro: o preset \"Não existe\" não existe. Veja a lista com lamina presets."
    );
}
