//! Espelho do código no repositório público `lamina-codigo`: só o código vai; estudo, specs e
//! documentos ficam no privado. `cargo xtask espelho <pasta>` copia o código para a pasta (um
//! clone do público); `cargo xtask espelho --trazer <pasta>` traz o código de volta.

use std::fs;
use std::path::{Path, PathBuf};

/// O que é código: vai para o público e pode voltar dele.
pub const CODE: &[&str] = &[
    "crates",
    "xtask",
    ".cargo",
    ".github",
    "Cargo.toml",
    "Cargo.lock",
    "rust-toolchain.toml",
    "rustfmt.toml",
    "clippy.toml",
    "deny.toml",
    ".gitignore",
    ".gitattributes",
];

/// O README do público: o mínimo (o README completo é do privado).
pub const PUBLIC_README: &str = "# Lâmina\n\n\
Diagramador de álbuns fotográficos em Rust, para Windows, macOS e Linux. Projeto acadêmico.\n\n\
## Compilar\n\n\
```bash\ncargo run --release -p lamina-app   # a janela\ncargo xtask verify                  # formatação, lint, testes e checagens\n```\n\n\
No Linux, a janela precisa de `libfontconfig1-dev`, `libxkbcommon-dev` e `libwayland-dev`.\n\n\
## Licença\n\nApache-2.0 (arquivo `LICENSE`). A interface usa o Slint sob a licença Royalty-free 2.0.\n";

/// Caminhos de `CODE` relativos à raiz, arquivo por arquivo (sem `target`).
pub fn files(root: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    for entry in CODE {
        collect(root, &root.join(entry), &mut out);
    }
    out.sort();
    out
}

fn collect(root: &Path, path: &Path, out: &mut Vec<PathBuf>) {
    if path.is_dir() {
        if path.file_name().is_some_and(|n| n == "target") {
            return;
        }
        let Ok(entries) = fs::read_dir(path) else {
            return;
        };
        for e in entries.flatten() {
            collect(root, &e.path(), out);
        }
    } else if path.is_file()
        && let Ok(rel) = path.strip_prefix(root)
    {
        out.push(rel.to_path_buf());
    }
}

/// Copia o código de `from` para `to`, apagando em `to` o código que não existe mais em `from`
/// (fora de `CODE` nada é tocado: `.git`, `LICENSE` e o resto ficam).
pub fn copy_code(from: &Path, to: &Path) -> std::io::Result<usize> {
    for old in files(to) {
        if !from.join(&old).exists() {
            fs::remove_file(to.join(&old))?;
        }
    }
    let list = files(from);
    for rel in &list {
        let target = to.join(rel);
        if let Some(dir) = target.parent() {
            fs::create_dir_all(dir)?;
        }
        fs::copy(from.join(rel), target)?;
    }
    Ok(list.len())
}

pub fn run(args: &[String]) -> bool {
    let (back, folder) = match args {
        [flag, folder] if flag == "--trazer" => (true, folder),
        [folder] => (false, folder),
        _ => {
            eprintln!("uso: cargo xtask espelho [--trazer] <pasta do clone do lamina-codigo>");
            return false;
        }
    };
    let (here, there) = (Path::new("."), Path::new(folder));
    if !there.join(".git").exists() {
        eprintln!("  ✘ {folder} não é um clone git");
        return false;
    }
    let result = if back {
        copy_code(there, here)
    } else {
        copy_code(here, there)
            .and_then(|n| fs::write(there.join("README.md"), PUBLIC_README).map(|()| n))
    };
    match result {
        Ok(n) => {
            println!("  espelho: {n} arquivo(s) copiado(s)");
            true
        }
        Err(e) => {
            eprintln!("  ✘ espelho: {e}");
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Proveta: documento nunca vai para o público; código apagado some de lá.
    #[test]
    fn so_o_codigo_vai_e_o_apagado_some() {
        let (a, b) = (tempfile::tempdir().unwrap(), tempfile::tempdir().unwrap());
        for f in [
            "crates/x/src/lib.rs",
            "Cargo.toml",
            "specs/s.md",
            "docs/estudo/e.md",
            "STATUS.md",
        ] {
            let p = a.path().join(f);
            fs::create_dir_all(p.parent().unwrap()).unwrap();
            fs::write(p, "x").unwrap();
        }
        fs::create_dir_all(b.path().join("crates/velho")).unwrap();
        fs::write(b.path().join("crates/velho/a.rs"), "x").unwrap();
        fs::write(b.path().join("LICENSE"), "apache").unwrap();
        copy_code(a.path(), b.path()).unwrap();
        assert!(b.path().join("crates/x/src/lib.rs").exists());
        assert!(!b.path().join("specs").exists() && !b.path().join("docs").exists());
        assert!(!b.path().join("STATUS.md").exists());
        assert!(
            !b.path().join("crates/velho/a.rs").exists(),
            "apagado no privado"
        );
        assert!(
            b.path().join("LICENSE").exists(),
            "fora do código não é tocado"
        );
    }
}
