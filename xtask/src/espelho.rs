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

/// O selo "Made with Slint" na página de onde se baixa o Lamina: é a atribuição que a licença
/// royalty-free do Slint pede (opção 2b; ADR 0002, adendo). Vai no README do público e nas
/// notas de cada versão (`release.yml`).
pub const SLINT_BADGE: &str = "[![Made with Slint](https://raw.githubusercontent.com/slint-ui/slint/master/logo/MadeWithSlint-logo-whitebg.png)](https://slint.dev)";

/// O README do público: o mínimo (o README completo é do privado).
pub const PUBLIC_README: &str = "# Lamina\n\n\
Diagramador de álbuns fotográficos em Rust, para Windows, macOS e Linux. Projeto acadêmico.\n\n\
## Instalar\n\nBaixe o instalador do seu sistema na [página de versões](https://github.com/alberthpalhares/lamina-codigo/releases)\n(`Lamina-win-Setup.exe`, `Lamina-osx-Setup.pkg` ou `Lamina.AppImage`). Depois de instalado, o\nLamina avisa sozinho quando há versão nova.\n\nPelo terminal, sem o aviso de segurança do Windows e do Mac (o Lamina não é assinado; o aviso\nsó não aparece, e o antivírus continua examinando):\n\n| Sistema | Comando |\n|---|---|\n| Windows (PowerShell) | `irm https://raw.githubusercontent.com/alberthpalhares/lamina-codigo/main/crates/lamina-app/dist/instalar.ps1 \\| iex` |\n| Mac e Linux | `curl -fsSL https://github.com/alberthpalhares/lamina-codigo/releases/latest/download/instalar.sh \\| sh` |\n\nPara a versão de teste mais nova: `$env:LAMINA_BETA = 1` antes do comando no Windows, ou\n`sh -s -- --beta` no lugar de `sh` no Mac e no Linux.\n\n\
## Usar com o Claude Desktop\n\nBaixe `Lamina-Windows.mcpb` ou `Lamina-macOS.mcpb` na mesma página e dê dois cliques: o Claude\nDesktop instala o Lamina e pergunta as pastas dos álbuns. **Com \"Permitir que a IA veja as\nfotos\" ligada, as prévias das lâminas e as miniaturas das fotos vão para o provedor da IA** (no\nClaude, a Anthropic). São fotos de clientes (LGPD): desligue a opção se o contrato pedir; a IA\ncontinua só com os dados das fotos e das lâminas.\n\n\
## Compilar\n\n\
```bash\ncargo run --release -p lamina-app   # a janela\ncargo xtask verify                  # formatação, lint, testes e checagens\n```\n\n\
No Linux, a janela precisa de `libfontconfig1-dev`, `libxkbcommon-dev` e `libwayland-dev`.\n\n\
## Licença\n\nApache-2.0 (arquivo `LICENSE`). A interface usa o Slint sob a licença Royalty-free 2.0. \
As fontes em `crates/lamina-render/fonts/` têm a licença SIL OFL 1.1 (o `OFL.txt` fica na pasta \
de cada uma).\n";

/// O README do público como vai para o clone: o texto e o selo do Slint no fim.
pub fn public_readme() -> String {
    format!("{PUBLIC_README}\n{SLINT_BADGE}\n")
}

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

/// O que pode existir no público além do código: o README mínimo, a licença, o git e o `target`.
const PUBLIC_EXTRA: &[&str] = &["README.md", "LICENSE", ".git", "target"];

/// Entradas do topo de `dest` que não são código nem os extras do público: um `specs/` ou
/// `docs/` que entrou por engano fica lá para sempre, porque a cópia só olha o código
/// (auditoria do Marco 3, M9).
pub fn strays(dest: &Path) -> Vec<String> {
    let Ok(entries) = fs::read_dir(dest) else {
        return Vec::new();
    };
    let mut out: Vec<String> = entries
        .flatten()
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .filter(|n| !CODE.contains(&n.as_str()) && !PUBLIC_EXTRA.contains(&n.as_str()))
        .collect();
    out.sort();
    out
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
    if !back {
        let extra = strays(there);
        if !extra.is_empty() {
            eprintln!(
                "  ✘ espelho: {folder} tem o que não é código (apague à mão e confira): {}",
                extra.join(", ")
            );
            return false;
        }
    }
    let result = if back {
        copy_code(there, here)
    } else {
        copy_code(here, there)
            .and_then(|n| fs::write(there.join("README.md"), public_readme()).map(|()| n))
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

    /// Proveta do M9 do Marco 3: um documento que já estava no público é apontado.
    #[test]
    fn documento_que_ja_estava_no_publico_e_apontado() {
        let dir = tempfile::tempdir().unwrap();
        for f in ["Cargo.toml", "README.md", "LICENSE"] {
            fs::write(dir.path().join(f), "x").unwrap();
        }
        fs::create_dir_all(dir.path().join("crates")).unwrap();
        assert!(strays(dir.path()).is_empty());
        fs::create_dir_all(dir.path().join("specs")).unwrap();
        fs::write(dir.path().join("STATUS.md"), "x").unwrap();
        assert_eq!(strays(dir.path()), ["STATUS.md", "specs"]);
    }

    /// Trava da atribuição do Slint (ADR 0002, adendo): o selo não pode sumir do README.
    #[test]
    fn readme_publico_tem_o_selo_do_slint() {
        assert!(public_readme().ends_with(&format!("\n{SLINT_BADGE}\n")));
        assert!(SLINT_BADGE.contains("MadeWithSlint") && SLINT_BADGE.contains("slint.dev"));
    }

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
