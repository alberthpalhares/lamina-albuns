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
        "15x15cm", "20x20cm", "25x25cm", "30x30cm", "21x15cm", "25x20cm", "30x20cm", "30x25cm",
        "35x30cm", "40x30cm", "45x30cm", "15x21cm", "20x25cm", "20x30cm", "25x30cm",
    ] {
        assert!(out.contains(name), "{name}: {out}");
    }
    // Um nome repetido entre encadernadoras aparece com a encadernadora (PR-B13).
    assert!(out.contains("25x20cm (ProAlbuns)"), "{out}");

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
        "20x30cm",
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
