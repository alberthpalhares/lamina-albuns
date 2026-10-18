//! MD-B11 (spec `modelos`): `lamina build` só usa os modelos do fotógrafo com `--modelos`.

#![allow(clippy::unwrap_used)]

mod common;

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use common::{Base, text};
use lamina_core::Project;
use lamina_editor::{MODELS_FILE, ModelsStore, PREFS_FILE, PrefsStore, model_from_spread};

/// A pasta de configuração que o `lamina` vai usar com `HOME`, `APPDATA` e `XDG_CONFIG_HOME`
/// apontando para `root` (a mesma conta de `config_dir`).
fn config_in(root: &Path) -> PathBuf {
    if cfg!(target_os = "macos") {
        root.join("Library")
            .join("Application Support")
            .join("Lamina")
    } else {
        root.join("Lamina")
    }
}

fn build(base: &Base, saida: &str, extra: &[&str]) -> Output {
    let config = base.path("config");
    let cache = base.path("cache");
    let mut args = vec!["build", "fotos", "--saida", saida, "--nome", "album"];
    args.extend([
        "--laminas",
        "4",
        "--largura-cm",
        "5",
        "--altura-cm",
        "5",
        "--dpi",
        "72",
    ]);
    args.extend(["--cache", cache.to_str().unwrap()]);
    args.extend(extra);
    Command::new(env!("CARGO_BIN_EXE_lamina"))
        .current_dir(base.path(""))
        .env("HOME", &config)
        .env("APPDATA", &config)
        .env("XDG_CONFIG_HOME", &config)
        .args(&args)
        .output()
        .unwrap()
}

fn project(base: &Base, saida: &str) -> Project {
    let json = std::fs::read_to_string(base.path(saida).join("album.lamina")).unwrap();
    Project::from_json(&json).unwrap()
}

#[test]
fn md_b11_linha_de_comando() {
    let base = Base::with_photos(12, (300, 200));
    let first = build(&base, "antes", &[]);
    assert!(first.status.success(), "{}", text(&first.stderr));
    let before = project(&base, "antes");

    // A lâmina 1 vira o modelo "Abertura", favoritado.
    let dir = config_in(&base.path("config"));
    let model = model_from_spread(&before, 0, "Abertura").unwrap();
    ModelsStore::load(&dir.join(MODELS_FILE))
        .save(model)
        .unwrap();
    let mut prefs = PrefsStore::load(&dir.join(PREFS_FILE));
    prefs.prefs.favorites.insert("modelo:Abertura".into());
    prefs.save().unwrap();

    let plain = build(&base, "sem", &[]);
    assert!(plain.status.success(), "{}", text(&plain.stderr));
    assert_eq!(
        project(&base, "sem").spreads,
        before.spreads,
        "sem --modelos, o mesmo álbum"
    );

    let with = build(&base, "com", &["--modelos"]);
    assert!(with.status.success(), "{}", text(&with.stderr));
    let used = project(&base, "com");
    assert!(
        used.spreads
            .iter()
            .any(|s| s.model.as_deref() == Some("modelo:Abertura")),
        "{}",
        text(&with.stdout)
    );
}
