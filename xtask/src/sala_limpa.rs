//! Trava da sala limpa (AGENTS.md, regra 6): o código do produto não pode conter a marca do
//! software de referência, e o repositório não pode guardar arquivos no formato dele.
//!
//! Só o estudo (`docs/estudo/`) pode citar o nome; o resto do repositório não (auditoria do
//! Marco 1, M12: antes só `crates/` era lido).

use std::fs;
use std::path::Path;

/// Termos proibidos no código do produto, montados em partes para este arquivo não se acusar.
fn forbidden_terms() -> Vec<String> {
    vec![["smart", "albums"].concat(), ["pix", "ellu"].concat()]
}

/// Extensões de arquivo do software de referência que nunca entram no repositório.
const FORBIDDEN_EXTENSIONS: &[&str] = &["saproj"];

/// Termos proibidos encontrados em `text` (sem diferenciar maiúsculas, e também quando escritos
/// com espaço, hífen, ponto ou sublinhado no meio; auditoria do Marco 2, M3).
pub fn terms_in(text: &str) -> Vec<String> {
    let lower: String = text
        .to_lowercase()
        .chars()
        .filter(|c| !matches!(c, ' ' | '-' | '.' | '_'))
        .collect();
    forbidden_terms()
        .into_iter()
        .filter(|t| lower.contains(t.as_str()))
        .collect()
}

pub fn has_forbidden_extension(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| FORBIDDEN_EXTENSIONS.contains(&e.to_lowercase().as_str()))
}

fn walk(dir: &Path, check_text: bool, problems: &mut Vec<String>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for path in entries.flatten().map(|e| e.path()) {
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        if path.is_dir() {
            if !matches!(name, "target" | ".git") {
                let study = path.ends_with(Path::new("docs").join("estudo"));
                walk(&path, check_text && !study, problems);
            }
            continue;
        }
        if has_forbidden_extension(&path) {
            problems.push(format!(
                "{}: arquivo no formato do software de referência",
                path.display()
            ));
        }
        if check_text && let Ok(text) = fs::read_to_string(&path) {
            for term in terms_in(&text) {
                problems.push(format!("{}: contém \"{term}\"", path.display()));
            }
        }
    }
}

pub fn run() -> bool {
    let mut problems = Vec::new();
    walk(Path::new("."), true, &mut problems);
    problems.sort();
    problems.dedup();
    for p in &problems {
        eprintln!("  ✘ {p}");
    }
    if problems.is_empty() {
        println!("  sala limpa ok");
    }
    problems.is_empty()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Proveta: a marca no código tem que ser apontada.
    #[test]
    fn marca_no_codigo_e_apontada() {
        let code = format!("// layout copiado do {}", ["Smart", "Albums"].concat());
        assert_eq!(terms_in(&code).len(), 1);
        assert!(terms_in("// layout nosso, feito do zero").is_empty());
        // Proveta do M3 do Marco 2: com espaço ou hífen no meio também.
        for sep in [" ", "-", "_", "."] {
            let split = format!("// como no {}", ["Smart", sep, "Albums"].concat());
            assert_eq!(terms_in(&split).len(), 1, "{split}");
        }
    }

    #[test]
    fn extensao_de_projeto_da_referencia_e_proibida() {
        assert!(has_forbidden_extension(Path::new("estudo/teste.SAPROJ")));
        assert!(!has_forbidden_extension(Path::new("album.json")));
    }
}
