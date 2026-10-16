//! Portão de entrada de fase (T3): lista tudo o que outras partes do projeto deixaram para a
//! fase que está abrindo, com arquivo e linha.
//!
//! Uso: `cargo xtask portao "Marco 1" "Semanas 4" "Semana 4"`. A busca considera linhas com
//! uma seta de alocação (`→`) que contenham algum dos termos.

use std::fs;
use std::path::Path;

/// Linhas de `text` com `→` e algum dos `terms`, como (número da linha, texto).
pub fn matches(text: &str, terms: &[String]) -> Vec<(usize, String)> {
    text.lines()
        .enumerate()
        .filter(|(_, line)| line.contains('→') && terms.iter().any(|t| line.contains(t.as_str())))
        .map(|(i, line)| (i + 1, line.trim().to_owned()))
        .collect()
}

fn walk(dir: &Path, terms: &[String], found: &mut Vec<String>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    let mut paths: Vec<_> = entries.flatten().map(|e| e.path()).collect();
    paths.sort();
    for path in paths {
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        if path.is_dir() {
            if !matches!(name, "target" | ".git" | "spikes" | "xtask") {
                walk(&path, terms, found);
            }
        } else if path.extension().is_some_and(|e| e == "md" || e == "rs") {
            let text = fs::read_to_string(&path).unwrap_or_default();
            for (line, content) in matches(&text, terms) {
                found.push(format!("{}:{line}: {content}", path.display()));
            }
        }
    }
}

pub fn run(terms: &[String]) -> bool {
    if terms.is_empty() {
        eprintln!("uso: cargo xtask portao \"Marco 1\" [outros termos]");
        return false;
    }
    let mut found = Vec::new();
    walk(Path::new("."), terms, &mut found);
    println!("Itens alocados para {}:", terms.join(" / "));
    for f in &found {
        println!("  {f}");
    }
    if found.is_empty() {
        println!("  (nenhum)");
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn acha_so_linhas_com_seta_e_termo() {
        let text = "a\n`Alocação: → Semanas 4–7`\nMarco 1 sem seta\n→ Marco 2\n";
        let terms = vec!["Semanas 4".to_owned(), "Marco 1".to_owned()];
        assert_eq!(
            matches(text, &terms),
            [(2, "`Alocação: → Semanas 4–7`".to_owned())]
        );
    }
}
