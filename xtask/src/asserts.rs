//! Trava de assertivas frouxas (auditoria da Fundação, achado A3): `assert!(x.is_err())` passa
//! com qualquer erro, até um que não tem nada a ver com o teste. Confira a variante exata com
//! `matches!(x, Err(Erro::Variante { .. }))`. Exceção consciente: comente `// qualquer erro`.

use std::fs;
use std::path::Path;

/// `Err(_)` dentro de `matches!` e `is_err_and(|_|` também aceitam qualquer erro (auditoria do
/// Marco 2, M3), assim como `Err(..)` e `.err().is_some()` (auditoria do Marco 3, M9).
const LOOSE: &[&str] = &[
    ".is_err()",
    ".is_ok()",
    "Err(_)",
    "Err(..)",
    ".err().is_some()",
    ".is_err_and(|_|",
];
const ALLOW: &str = "// qualquer erro";

/// Linhas (1-based) com assertiva frouxa sem a marca de exceção. Vale também quando a
/// assertiva tem mensagem ou quebra a linha (`assert!(` na linha de cima; auditoria do Marco 1,
/// M2).
pub fn loose_lines(text: &str) -> Vec<usize> {
    let lines: Vec<&str> = text.lines().collect();
    lines
        .iter()
        .enumerate()
        .filter(|(i, l)| {
            let in_assert = l.contains("assert") || (*i > 0 && lines[i - 1].contains("assert"));
            in_assert && LOOSE.iter().any(|p| l.contains(p)) && !l.contains(ALLOW)
        })
        .map(|(i, _)| i + 1)
        .collect()
}

fn walk(dir: &Path, problems: &mut Vec<String>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for path in entries.flatten().map(|e| e.path()) {
        if path.is_dir() {
            walk(&path, problems);
        } else if path.extension().is_some_and(|e| e == "rs") {
            let text = fs::read_to_string(&path).unwrap_or_default();
            for line in loose_lines(&text) {
                problems.push(format!("{}:{line}", path.display()));
            }
        }
    }
}

pub fn run() -> bool {
    let mut problems = Vec::new();
    walk(Path::new("crates"), &mut problems);
    for p in &problems {
        eprintln!("  ✘ {p}: assertiva frouxa; confira o erro exato com matches!");
    }
    if problems.is_empty() {
        println!("  assertivas ok");
    }
    problems.is_empty()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Proveta: `is_err()` sem conferir o erro tem que reprovar.
    #[test]
    fn is_err_solto_e_apontado() {
        let code = "assert!(f().is_err());\nassert!(matches!(f(), Err(E::X)));\n";
        assert_eq!(loose_lines(code), [1]);
    }

    /// Proveta do achado M2: com mensagem ou em duas linhas também reprova.
    #[test]
    fn com_mensagem_ou_quebrado_e_apontado() {
        assert_eq!(loose_lines("assert!(f().is_err(), \"x\");"), [1]);
        assert_eq!(loose_lines("assert!(\n    f().is_ok(),\n);"), [2]);
    }

    /// Proveta do M3 do Marco 2: `Err(_)` e `is_err_and(|_|` aceitam qualquer erro.
    #[test]
    fn erro_curinga_e_apontado() {
        assert_eq!(loose_lines("assert!(matches!(f(), Err(_)));"), [1]);
        assert_eq!(loose_lines("assert!(f().is_err_and(|_| true));"), [1]);
        assert_eq!(loose_lines("assert!(matches!(f(), Err(..)));"), [1]);
        assert_eq!(loose_lines("assert!(f().err().is_some());"), [1]);
        assert!(loose_lines("assert!(matches!(f(), Err(E::Nome { .. })));").is_empty());
    }

    #[test]
    fn excecao_comentada_passa() {
        assert!(loose_lines("assert!(f().is_err()); // qualquer erro serve aqui").is_empty());
    }
}
