//! Trava das specs (T3): toda spec com status "implementada" precisa de pelo menos um teste por
//! cenário BDD. O cenário `PL-B1` é cobrado por uma função `fn pl_b1_…` marcada com `#[test]`
//! (o `_` depois do número evita que `ab_b10` cubra o `ab_b1`; auditoria do Marco 1, M1).

use std::fs;
use std::path::{Path, PathBuf};

/// IDs dos cenários de uma spec (`CENÁRIO PL-B1: …` → `PL-B1`).
pub fn scenario_ids(spec: &str) -> Vec<String> {
    spec.lines()
        .filter_map(|line| {
            let rest = line.trim_start().strip_prefix("CENÁRIO ")?;
            let id = rest.split(':').next()?.trim();
            id.contains("-B").then(|| id.to_owned())
        })
        .collect()
}

/// `true` se a linha de status da spec diz "implementada".
pub fn is_implemented(spec: &str) -> bool {
    spec.lines()
        .find(|l| l.contains("**Status:**"))
        .is_some_and(|l| l.to_lowercase().contains("implementada"))
}

/// Cenários sem teste: procura `fn <id_minúsculo>_` com `#[test]` entre os atributos de cima
/// (pulando comentários de documentação). Um `#[ignore]` ali não conta: teste que não roda não
/// cobre o cenário (auditoria do Marco 2, M3).
pub fn missing_tests(spec: &str, sources: &str) -> Vec<String> {
    let lines: Vec<&str> = sources.lines().map(str::trim).collect();
    scenario_ids(spec)
        .into_iter()
        .filter(|id| {
            let prefix = format!("fn {}_", id.to_lowercase().replace('-', "_"));
            !lines.iter().enumerate().any(|(i, line)| {
                let attributes: Vec<&&str> = lines[..i]
                    .iter()
                    .rev()
                    .filter(|l| !l.is_empty() && !l.starts_with("///"))
                    .take_while(|l| l.starts_with("#["))
                    .collect();
                line.starts_with(&prefix)
                    && attributes.iter().any(|l| l.contains("#[test]"))
                    && !attributes.iter().any(|l| l.contains("#[ignore"))
            })
        })
        .collect()
}

fn rust_sources(dir: &Path, out: &mut String) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for path in entries.flatten().map(|e| e.path()) {
        if path.is_dir() {
            rust_sources(&path, out);
        } else if path.extension().is_some_and(|e| e == "rs") {
            out.push_str(&fs::read_to_string(&path).unwrap_or_default());
            out.push('\n');
        }
    }
}

pub fn run() -> bool {
    // No repositório público (só o código) não há specs: a checagem roda só no privado.
    if !Path::new("specs").is_dir() {
        println!("  specs: sem pasta specs/ (repositório só de código), checagem pulada");
        return true;
    }
    let mut sources = String::new();
    rust_sources(Path::new("crates"), &mut sources);
    // As travas do próprio xtask também são cenários (ex.: DS-B16, `cargo xtask versao`).
    rust_sources(Path::new("xtask"), &mut sources);
    let mut specs: Vec<PathBuf> = fs::read_dir("specs")
        .map(|d| d.flatten().map(|e| e.path()).collect())
        .unwrap_or_default();
    specs.retain(|p| p.extension().is_some_and(|e| e == "md") && !p.ends_with("README.md"));
    specs.sort();
    let mut ok = true;
    let mut implemented = 0;
    for spec in &specs {
        let text = fs::read_to_string(spec).unwrap_or_default();
        if !is_implemented(&text) {
            continue;
        }
        implemented += 1;
        for id in missing_tests(&text, &sources) {
            eprintln!("  ✘ {}: cenário {id} sem teste", spec.display());
            ok = false;
        }
    }
    if ok {
        println!(
            "  specs ok ({} spec(s), {implemented} implementada(s))",
            specs.len()
        );
    }
    ok
}

#[cfg(test)]
mod tests {
    use super::*;

    const SPEC: &str =
        "- **Status:** implementada\n\nCENÁRIO PL-B1: caminho feliz\nCENÁRIO PL-B2: erro\n";

    #[test]
    fn le_os_ids_dos_cenarios() {
        assert_eq!(scenario_ids(SPEC), ["PL-B1", "PL-B2"]);
    }

    #[test]
    fn status_implementada() {
        assert!(is_implemented(SPEC));
        assert!(!is_implemented("- **Status:** aguardando aprovação"));
    }

    /// Proveta: cenário sem teste tem que aparecer.
    #[test]
    fn cenario_sem_teste_e_apontado() {
        let sources = "#[test]\nfn pl_b1_quarenta_fotos() {}";
        assert_eq!(missing_tests(SPEC, sources), ["PL-B2"]);
    }

    /// Proveta do achado M1: o teste do B10 não pode cobrir o B1.
    /// Proveta do M3 do Marco 2: teste ignorado não cobre o cenário.
    #[test]
    fn teste_ignorado_nao_conta() {
        for sources in [
            "#[test]\n#[ignore]\nfn pl_b1_x() {}\n#[test]\nfn pl_b2_y() {}",
            "#[ignore = \"lento\"]\n#[test]\nfn pl_b1_x() {}\n#[test]\nfn pl_b2_y() {}",
        ] {
            assert_eq!(missing_tests(SPEC, sources), ["PL-B1"], "{sources}");
        }
        let with_doc =
            "/// doc\n#[test]\n#[allow(clippy::x)]\nfn pl_b1_x() {}\n#[test]\nfn pl_b2_y() {}";
        assert!(missing_tests(SPEC, with_doc).is_empty());
    }

    #[test]
    fn b1_nao_e_coberto_pelo_b10() {
        let spec = "CENÁRIO AB-B1: um\nCENÁRIO AB-B10: dez\n";
        let sources = "#[test]\nfn ab_b10_x() {}";
        assert_eq!(missing_tests(spec, sources), ["AB-B1"]);
    }

    #[test]
    fn funcao_sem_test_nao_conta() {
        let sources = "fn pl_b1_ajudante() {}\n#[test]\n/// Documentação.\nfn pl_b2_erro() {}";
        assert_eq!(missing_tests(SPEC, sources), ["PL-B1"]);
    }
}
