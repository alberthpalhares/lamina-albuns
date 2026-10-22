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

/// Cenários marcados "(só no Mac)", "(só no Windows)" ou "(só no Linux)" na linha do título: só
/// esses aceitam um teste que roda num sistema só (`#[cfg(target_os = …)]`, `#[cfg(windows)]`).
pub fn platform_only(spec: &str) -> Vec<String> {
    spec.lines()
        .filter(|line| line.contains("(só no "))
        .filter_map(|line| scenario_ids(line).into_iter().next())
        .collect()
}

/// Atributo que faz o teste rodar num sistema só: no Windows, a porta local não o roda, e o CI
/// dos PRs (só Linux) também não (auditoria da Rodada 0b, achado do MA-B6).
fn is_platform_gate(line: &str) -> bool {
    line.contains("#[cfg(target_os")
        || line.contains("#[cfg(windows")
        || line.contains("#[cfg(unix")
}

/// `#[test]`, ou o teste assíncrono do `tokio` (`#[tokio::test]`, nos testes do servidor MCP).
fn is_test_attribute(line: &str) -> bool {
    line.contains("#[test]") || line.contains("#[tokio::test")
}

/// Cenários sem teste: procura `fn <id_minúsculo>_` com `#[test]` entre os atributos de cima
/// (pulando comentários de documentação). Um `#[ignore]` ali não conta: teste que não roda não
/// cobre o cenário (auditoria do Marco 2, M3). Um teste de um sistema só cobre só o cenário marcado
/// para aquele sistema ([`platform_only`]).
pub fn missing_tests(spec: &str, sources: &str) -> Vec<String> {
    let lines: Vec<&str> = sources.lines().map(str::trim).collect();
    let marked = platform_only(spec);
    scenario_ids(spec)
        .into_iter()
        .filter(|id| {
            let gate_ok = marked.contains(id);
            let prefix = format!("fn {}_", id.to_lowercase().replace('-', "_"));
            !lines.iter().enumerate().any(|(i, line)| {
                let attributes: Vec<&&str> = lines[..i]
                    .iter()
                    .rev()
                    .filter(|l| !l.is_empty() && !l.starts_with("///"))
                    .take_while(|l| l.starts_with("#["))
                    .collect();
                line.trim_start_matches("async ").starts_with(&prefix)
                    && attributes.iter().any(|l| is_test_attribute(l))
                    && !attributes.iter().any(|l| l.contains("#[ignore"))
                    && (gate_ok || !attributes.iter().any(|l| is_platform_gate(l)))
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

    /// Os testes do servidor MCP são assíncronos (spec IA, IA-B17 e IA-B19).
    #[test]
    fn teste_assincrono_do_tokio_conta() {
        let spec = "CENÁRIO IA-B17: protocolo\nCENÁRIO IA-B19: recursos";
        let sources = "#[tokio::test]\nasync fn ia_b17_x() {}\n#[tokio::test(flavor = \"multi_thread\")]\nasync fn ia_b19_y() {}";
        assert!(missing_tests(spec, sources).is_empty());
        assert_eq!(
            missing_tests(spec, "async fn ia_b17_x() {}"),
            ["IA-B17", "IA-B19"]
        );
    }

    #[test]
    fn b1_nao_e_coberto_pelo_b10() {
        let spec = "CENÁRIO AB-B1: um\nCENÁRIO AB-B10: dez\n";
        let sources = "#[test]\nfn ab_b10_x() {}";
        assert_eq!(missing_tests(spec, sources), ["AB-B1"]);
    }

    /// Achado da auditoria da Rodada 0b: um teste só do Mac não roda na porta local nem no CI
    /// dos PRs; só cobre o cenário marcado "(só no Mac)".
    #[test]
    fn teste_de_um_sistema_so_cobre_so_o_cenario_marcado() {
        let spec = "CENÁRIO MA-B9: todo sistema
CENÁRIO MA-B10: instalar (só no Mac)
";
        let sources = "#[cfg(target_os = \"macos\")]
#[test]
fn ma_b9_x() {}
                       #[cfg(target_os = \"macos\")]
#[test]
fn ma_b10_y() {}";
        assert_eq!(platform_only(spec), ["MA-B10"]);
        assert_eq!(missing_tests(spec, sources), ["MA-B9"]);
        let windows = "#[cfg(windows)]
#[test]
fn ma_b9_x() {}
#[test]
fn ma_b10_y() {}";
        assert_eq!(missing_tests(spec, windows), ["MA-B9"]);
    }

    #[test]
    fn funcao_sem_test_nao_conta() {
        let sources = "fn pl_b1_ajudante() {}\n#[test]\n/// Documentação.\nfn pl_b2_erro() {}";
        assert_eq!(missing_tests(SPEC, sources), ["PL-B1"]);
    }
}
