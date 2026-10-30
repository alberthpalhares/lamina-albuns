//! Testes do repositório para a fundação do design (spec DF): a interface usa a Inter do
//! `lamina-render`, e a exportação não depende de nada da interface (os tokens não mudam a
//! impressão).

use std::fs;
use std::path::{Path, PathBuf};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..")
}

#[test]
fn df_b12_a_interface_usa_a_inter() {
    let app = fs::read_to_string(root().join("crates/lamina-app/ui/app.slint")).unwrap();
    assert!(app.contains("default-font-family: \"Inter\";"));
    assert!(app.contains("import \"../../lamina-render/fonts/inter/Inter.ttf\";"));
    assert!(
        root()
            .join("crates/lamina-render/fonts/inter/Inter.ttf")
            .is_file()
    );
}

#[test]
fn df_b13_a_lamina_e_a_exportacao_nao_dependem_da_interface() {
    for krate in ["lamina-core", "lamina-render", "lamina-export"] {
        let manifest = root().join("crates").join(krate).join("Cargo.toml");
        let text = fs::read_to_string(&manifest).unwrap();
        assert!(!text.contains("slint"), "{krate} depende do Slint");
        assert!(
            !text.contains("lamina-app"),
            "{krate} depende do lamina-app"
        );
    }
}

#[test]
fn df_b16_as_faixas_usam_o_token_de_faixa() {
    let text = fs::read_to_string(root().join("crates/lamina-app/ui/strip_header.slint")).unwrap();
    for strip in ["spreads", "timeline", "photos"] {
        assert!(text.contains(&format!("out property <color> {strip}: Theme.strip;")));
    }
}
