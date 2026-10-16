//! Alerta graduado de tamanho de arquivo. Nunca reprova: só avisa.
//!
//! Em Rust os testes ficam no mesmo arquivo, depois de `#[cfg(test)]`. Por isso o código e
//! os testes são medidos separadamente, cada um contra o seu alvo.

use std::fs;
use std::path::{Path, PathBuf};

/// Alvo de linhas para o código de um arquivo (antes de `#[cfg(test)]`).
pub const CODE_TARGET: usize = 200;
/// Alvo de linhas para o bloco de testes de um arquivo.
pub const TEST_TARGET: usize = 300;

/// Faixa de alerta pelo percentual sobre o alvo; `None` = dentro do alvo.
pub fn band(lines: usize, target: usize) -> Option<&'static str> {
    let percent = lines * 100 / target.max(1);
    match percent {
        0..=100 => None,
        101..=110 => Some("aviso"),
        111..=130 => Some("atenção"),
        131..=150 => Some("alto"),
        _ => Some("crítico"),
    }
}

/// `true` para arquivos que só contêm testes (`tests.rs`, ou dentro de uma pasta `tests/`).
pub fn is_test_file(path: &Path) -> bool {
    path.file_name().is_some_and(|n| n == "tests.rs")
        || path.components().any(|c| c.as_os_str() == "tests")
}

/// Separa as linhas de código e de teste de um arquivo-fonte.
pub fn split_counts(source: &str) -> (usize, usize) {
    let total = source.lines().count();
    let code = source
        .lines()
        .position(|line| line.trim_start().starts_with("#[cfg(test)]"))
        .unwrap_or(total);
    (code, total - code)
}

fn rust_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for path in entries.flatten().map(|e| e.path()) {
        if path.is_dir() && !path.ends_with("target") {
            rust_files(&path, out);
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            out.push(path);
        }
    }
}

pub fn run() {
    let mut files = Vec::new();
    rust_files(Path::new("crates"), &mut files);
    rust_files(Path::new("xtask"), &mut files);
    files.sort();
    let mut alerts = 0;
    for file in &files {
        let Ok(source) = fs::read_to_string(file) else {
            continue;
        };
        let (code, tests) = if is_test_file(file) {
            (0, source.lines().count())
        } else {
            split_counts(&source)
        };
        for (kind, lines, target) in [
            ("código", code, CODE_TARGET),
            ("testes", tests, TEST_TARGET),
        ] {
            if let Some(band) = band(lines, target) {
                alerts += 1;
                println!(
                    "  ⚠ {band}: {} — {kind} com {lines} linhas (alvo {target})",
                    file.display()
                );
            }
        }
    }
    if alerts == 0 {
        println!("  tamanhos ok ({} arquivos)", files.len());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn faixas_de_alerta() {
        assert_eq!(band(200, 200), None);
        assert_eq!(band(215, 200), Some("aviso"));
        assert_eq!(band(250, 200), Some("atenção"));
        assert_eq!(band(290, 200), Some("alto"));
        assert_eq!(band(300, 200), Some("alto"));
        assert_eq!(band(310, 200), Some("crítico"));
    }

    #[test]
    fn separa_codigo_de_testes() {
        let source = "fn a() {}\nfn b() {}\n#[cfg(test)]\nmod tests {\n}\n";
        assert_eq!(split_counts(source), (2, 3));
        assert_eq!(split_counts("fn a() {}\n"), (1, 0));
    }

    #[test]
    fn arquivo_so_de_testes_conta_como_teste() {
        assert!(is_test_file(Path::new("crates/x/src/plan/tests.rs")));
        assert!(is_test_file(Path::new("crates/x/tests/integracao.rs")));
        assert!(!is_test_file(Path::new("crates/x/src/plan/mod.rs")));
    }
}
