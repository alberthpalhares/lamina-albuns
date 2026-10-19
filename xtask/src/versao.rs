//! `cargo xtask versao <tag> [--notas <arquivo>]` (spec DS, regra 11): a release só segue se a
//! tag é a versão do `Cargo.toml` e o `CHANGELOG.md` tem a seção dela. Com `--notas`, grava a
//! seção num arquivo, para as notas da versão.

/// A versão de `[workspace.package]` no `Cargo.toml`.
pub fn workspace_version(cargo_toml: &str) -> Option<String> {
    let mut in_package = false;
    for line in cargo_toml.lines() {
        let line = line.trim();
        if line.starts_with('[') {
            in_package = line == "[workspace.package]";
        } else if in_package && let Some(rest) = line.strip_prefix("version") {
            let value = rest.trim_start().strip_prefix('=')?.trim();
            return Some(value.trim_matches('"').to_owned());
        }
    }
    None
}

/// O texto da seção `## [<versão>]` do CHANGELOG, até a próxima seção.
pub fn notes(changelog: &str, version: &str) -> Option<String> {
    let head = format!("## [{version}]");
    let start = changelog
        .lines()
        .position(|l| l.trim_start().starts_with(&head))?;
    let body: Vec<&str> = changelog
        .lines()
        .skip(start + 1)
        .take_while(|l| !l.starts_with("## "))
        .collect();
    let text = body.join("\n").trim().to_owned();
    (!text.is_empty()).then_some(text)
}

/// Confere a tag (`v0.2.0` ou `v0.2.0-beta.1`). `Ok` traz as notas da versão.
pub fn check(tag: &str, cargo_toml: &str, changelog: &str) -> Result<String, String> {
    let Some(version) = tag.strip_prefix('v') else {
        return Err(format!(
            "a tag {tag} precisa começar com \"v\" (ex.: v0.2.0)"
        ));
    };
    let Some(cargo) = workspace_version(cargo_toml) else {
        return Err("o Cargo.toml não tem a versão em [workspace.package]".to_owned());
    };
    if cargo != version {
        return Err(format!(
            "a tag é {version}, mas o Cargo.toml está na {cargo}: mude um dos dois"
        ));
    }
    notes(changelog, version).ok_or_else(|| {
        format!("o CHANGELOG.md não tem a seção \"## [{version}]\" com as novidades da versão")
    })
}

pub fn run(args: &[String]) -> bool {
    let (tag, out) = match args {
        [tag] => (tag, None),
        [tag, flag, file] if flag == "--notas" => (tag, Some(file)),
        _ => {
            eprintln!("uso: cargo xtask versao <tag> [--notas <arquivo>]");
            return false;
        }
    };
    let read = |p: &str| std::fs::read_to_string(p).unwrap_or_default();
    match check(tag, &read("Cargo.toml"), &read("CHANGELOG.md")) {
        Ok(text) => {
            if let Some(file) = out
                && let Err(e) = std::fs::write(file, text)
            {
                eprintln!("  ✘ versao: não deu para gravar {file}: {e}");
                return false;
            }
            println!("  versao ok ({tag})");
            true
        }
        Err(e) => {
            eprintln!("  ✘ versao: {e}");
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const CARGO: &str = "[workspace]\nmembers = []\n\n[workspace.package]\nversion = \"0.2.0\"\nedition = \"2024\"\n\n[package]\nversion = \"9.9.9\"\n";
    const LOG: &str = "# Changelog\n\n## [Não lançado]\n\n- algo\n\n## [0.2.0] - 2026-10-20\n\n- Atualização automática.\n- Instalar pelo terminal.\n\n## [0.1.0]\n\n- Início.\n";

    #[test]
    fn ds_b16_trava_da_versao() {
        assert_eq!(
            check("v0.2.0", CARGO, LOG).unwrap(),
            "- Atualização automática.\n- Instalar pelo terminal."
        );
        let other = check("v0.2.1", CARGO, LOG).unwrap_err();
        assert!(other.contains("Cargo.toml está na 0.2.0"), "{other}");
        let no_section = check("v0.2.0", CARGO, "# Changelog\n\n## [0.1.0]\n\n- Início.\n");
        assert!(
            no_section
                .unwrap_err()
                .contains("não tem a seção \"## [0.2.0]\""),
            "sem a seção"
        );
        let no_v = check("0.2.0", CARGO, LOG).unwrap_err();
        assert!(no_v.contains("começar com \"v\""), "{no_v}");
    }

    #[test]
    fn versao_de_teste_e_secao_vazia() {
        let beta = CARGO.replace("\"0.2.0\"", "\"0.3.0-beta.1\"");
        let log = "## [0.3.0-beta.1]\n\n- Teste.\n";
        assert_eq!(check("v0.3.0-beta.1", &beta, log).unwrap(), "- Teste.");
        assert_eq!(notes("## [0.3.0]\n\n## [0.2.0]\n- x\n", "0.3.0"), None);
    }
}
