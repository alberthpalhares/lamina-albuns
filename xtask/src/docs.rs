//! Documentos que o código precisa sustentar (auditoria do Marco 1):
//! - M6: toda dependência de `[workspace.dependencies]` aparece no `AGENTS.md` (seção Stack);
//! - M7: todo nome de código citado no glossário existe nos fontes, e nenhum termo se repete;
//! - M9: teste que mede tempo usa a folga única `budget(` (AGENTS.md, regra 13).

use std::collections::HashSet;
use std::fs;
use std::path::Path;

/// Nomes das dependências externas declaradas em `[workspace.dependencies]`.
pub fn workspace_deps(cargo_toml: &str) -> Vec<String> {
    let mut inside = false;
    let mut out = Vec::new();
    for line in cargo_toml.lines().map(str::trim) {
        if line.starts_with('[') {
            inside = line == "[workspace.dependencies]";
            continue;
        }
        if !inside || line.starts_with('#') {
            continue;
        }
        if let Some((name, _)) = line.split_once('=') {
            let name = name.trim();
            if !name.is_empty() && !name.starts_with("lamina-") {
                out.push(name.to_owned());
            }
        }
    }
    out
}

/// Dependências que o `AGENTS.md` não cita (entre crases ou não).
pub fn deps_missing_in(agents: &str, deps: &[String]) -> Vec<String> {
    deps.iter()
        .filter(|d| !agents.contains(d.as_str()))
        .cloned()
        .collect()
}

/// Problemas do glossário: nome de código que não existe nos fontes e termo repetido.
pub fn glossary_problems(glossary: &str, sources: &str) -> Vec<String> {
    let mut seen = HashSet::new();
    let mut out = Vec::new();
    for line in glossary.lines().filter(|l| l.starts_with("| ")) {
        let cols: Vec<&str> = line.split('|').map(str::trim).collect();
        let (Some(term), Some(code)) = (cols.get(1), cols.get(2)) else {
            continue;
        };
        if *term == "Português" || term.starts_with("---") {
            continue;
        }
        // "Pré-impressão (preflight)" e "Pré-impressão" são o mesmo termo.
        let key = term
            .split(" (")
            .next()
            .unwrap_or(term)
            .trim()
            .to_lowercase();
        if !seen.insert(key) {
            out.push(format!("termo repetido: \"{term}\""));
        }
        for ident in code_names(code) {
            if !has_word(sources, &ident) {
                out.push(format!("\"{term}\": `{ident}` não existe no código"));
            }
        }
    }
    out
}

/// Identificadores entre crases na coluna "No código" (`Family::Mosaic` → `Mosaic`,
/// `--paginas-minimas` → `paginas_minimas`). Extensões (`.lamina`) e textos com espaço não contam.
fn code_names(code: &str) -> Vec<String> {
    code.split('`')
        .skip(1)
        .step_by(2)
        .filter(|t| !t.contains(' ') && !t.starts_with('.'))
        .map(|t| match t.strip_prefix("--") {
            Some(flag) => flag.replace('-', "_"),
            None => t
                .rsplit("::")
                .next()
                .unwrap_or(t)
                .trim_end_matches("()")
                .to_owned(),
        })
        .collect()
}

/// `true` se `word` aparece inteira em `text` (sem letra, número ou `_` colados dos lados):
/// `Moment` não conta dentro de `Moments`.
fn has_word(text: &str, word: &str) -> bool {
    let ident = |c: char| c.is_alphanumeric() || c == '_';
    text.match_indices(word).any(|(i, _)| {
        let before = text[..i].chars().next_back();
        let after = text[i + word.len()..].chars().next();
        !before.is_some_and(ident) && !after.is_some_and(ident)
    })
}

/// `true` se o arquivo mede tempo num teste sem usar a folga única `budget(`.
pub fn timing_without_budget(text: &str) -> bool {
    text.contains("#[test]") && text.contains("Instant::now()") && !text.contains("budget(")
}

fn sources(dir: &Path, out: &mut Vec<(String, String)>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for path in entries.flatten().map(|e| e.path()) {
        if path.is_dir() {
            sources(&path, out);
        } else if path.extension().is_some_and(|e| e == "rs") {
            let text = fs::read_to_string(&path).unwrap_or_default();
            out.push((path.display().to_string(), text));
        }
    }
}

pub fn run() -> bool {
    // No repositório público (só o código) não há documentos: a checagem roda só no privado.
    if !Path::new("AGENTS.md").exists() {
        println!("  documentos: sem AGENTS.md (repositório só de código), checagem pulada");
        return true;
    }
    let read = |p: &str| fs::read_to_string(p).unwrap_or_default();
    let mut problems = Vec::new();
    for dep in deps_missing_in(&read("AGENTS.md"), &workspace_deps(&read("Cargo.toml"))) {
        problems.push(format!(
            "AGENTS.md não cita a dependência `{dep}` (seção Stack)"
        ));
    }
    let mut files = Vec::new();
    sources(Path::new("crates"), &mut files);
    let all: String = files
        .iter()
        .map(|(_, t)| t.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    for p in glossary_problems(&read("docs/glossario.md"), &all) {
        problems.push(format!("docs/glossario.md: {p}"));
    }
    for (path, text) in &files {
        if timing_without_budget(text) {
            problems.push(format!("{path}: teste de tempo sem `budget(` (regra 13)"));
        }
    }
    for p in &problems {
        eprintln!("  ✘ {p}");
    }
    if problems.is_empty() {
        println!("  documentos ok");
    }
    problems.is_empty()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Proveta do M6: dependência nova sem menção no AGENTS reprova.
    #[test]
    fn dependencia_sem_mencao_e_apontada() {
        let toml = "[workspace.dependencies]\nlamina-core = { path = \"x\" }\nfoo = \"1\"\n# nota\nbar = \"2\"\n[workspace.lints.rust]\nunsafe_code = \"forbid\"\n";
        assert_eq!(workspace_deps(toml), ["foo", "bar"]);
        assert_eq!(
            deps_missing_in("Stack: foo", &workspace_deps(toml)),
            ["bar"]
        );
    }

    /// Proveta do M7: nome que não existe e termo repetido reprovam.
    #[test]
    fn glossario_com_nome_inexistente_ou_repetido() {
        let glossary = "| Português | No código | Significado |\n|---|---|---|\n| Lâmina | `Spread` | x |\n| Momento | `Moment` | y |\n| lâmina (spread) | `--paginas-minimas`, `.lamina` | z |\n";
        let sources = "pub struct Spread; struct Moments; let paginas_minimas = 1;";
        assert_eq!(
            glossary_problems(glossary, sources),
            [
                "\"Momento\": `Moment` não existe no código",
                "termo repetido: \"lâmina (spread)\""
            ]
        );
    }

    /// Proveta do M9: teste de tempo sem a folga única reprova.
    #[test]
    fn teste_de_tempo_sem_budget_e_apontado() {
        let bad = "#[test]\nfn t() { let s = Instant::now(); assert!(s.elapsed() < Duration::from_millis(5)); }";
        assert!(timing_without_budget(bad));
        assert!(!timing_without_budget(
            &bad.replace("Duration::from_millis(5)", "budget(5)")
        ));
    }
}
