//! Trava das cores e da escala (spec DF, regras 1 e 6): nenhuma tela escreve cor, letra ou canto próprio. Fora de `ui/theme/`, os
//! `.slint` não têm `#rrggbb`, `rgb()`/`rgba()` nem `white`/`black`; só os tokens do `Theme` e o
//! `transparent`. Texto entre aspas (um exemplo de cor num campo, por exemplo) não conta.

use std::fs;
use std::path::Path;

/// Funções de cor e os nomes de cor que o Slint aceita, os mais comuns.
const NAMED: &[&str] = &[
    "rgb(",
    "rgba(",
    "hsv(",
    "white",
    "black",
    "red",
    "green",
    "blue",
    "gray",
    "grey",
    "yellow",
    "orange",
    "purple",
    "pink",
    "brown",
    "cyan",
    "magenta",
    "silver",
    "gold",
    "navy",
    "teal",
    "lime",
    "lightgray",
    "lightgrey",
    "darkgray",
    "darkgrey",
    "dimgray",
    "dimgrey",
];

/// O valor traz um número escrito à mão, mesmo dentro de uma condição (`x ? 12px : 11px`). Os cantos
/// só contam quando o valor começa no número: uma conta como `self.height / 2` é geometria.
fn is_literal(prop: &str, value: &str) -> bool {
    match prop {
        "border-radius:" => value.starts_with(|c: char| c.is_ascii_digit()),
        "font-weight:" => value
            .split(|c: char| !c.is_ascii_digit())
            .any(|n| n.len() == 3),
        _ => value
            .match_indices("px")
            .any(|(i, _)| value[..i].ends_with(|c: char| c.is_ascii_digit())),
    }
}

/// Letras, cantos e pesos escritos à mão em `line` (regra 6 da DF: só os tokens).
pub fn scale_literals(line: &str) -> Vec<String> {
    let code = outside_strings(line.split("//").next().unwrap_or(""));
    let mut found = Vec::new();
    for prop in ["font-size:", "border-radius:", "font-weight:"] {
        for (i, _) in code.match_indices(prop) {
            let value = code[i + prop.len()..]
                .split(';')
                .next()
                .unwrap_or("")
                .trim();
            if is_literal(prop, value) {
                found.push(format!("{prop} {value}"));
            }
        }
    }
    found
}

/// Cores escritas em `line` fora de comentários e de texto entre aspas.
pub fn colors_in(line: &str) -> Vec<String> {
    let code = outside_strings(line.split("//").next().unwrap_or(""));
    let mut found = Vec::new();
    for (i, _) in code.match_indices('#') {
        let hex: String = code[i + 1..]
            .chars()
            .take_while(char::is_ascii_hexdigit)
            .collect();
        let before = code[..i].chars().last();
        if matches!(hex.len(), 3 | 4 | 6 | 8) && !before.is_some_and(is_word) {
            found.push(format!("#{hex}"));
        }
    }
    for word in NAMED {
        found.extend(words(&code, word));
    }
    found
}

fn is_word(c: char) -> bool {
    c.is_alphanumeric() || c == '-' || c == '_' || c == '.'
}

/// Ocorrências de `word` que não fazem parte de um nome maior.
fn words(code: &str, word: &str) -> Vec<String> {
    code.match_indices(word)
        .filter(|(i, w)| {
            let before = code[..*i].chars().last();
            let after = code[i + w.len()..].chars().next();
            let open = w.ends_with('(');
            !before.is_some_and(is_word) && (open || !after.is_some_and(is_word))
        })
        .map(|(_, w)| w.trim_end_matches('(').to_owned())
        .collect()
}

/// A linha com o que está entre aspas trocado por espaços.
fn outside_strings(line: &str) -> String {
    let mut inside = false;
    line.chars()
        .map(|c| {
            if c == '"' {
                inside = !inside;
                return c;
            }
            if inside { ' ' } else { c }
        })
        .collect()
}

/// As cores soltas de cada `.slint` em `ui` (menos `ui/theme/`), como "arquivo:linha: cor".
pub fn loose_colors(ui: &Path) -> Vec<String> {
    let mut out = Vec::new();
    walk(ui, &ui.join("theme"), &mut out);
    out.sort();
    out
}

fn walk(dir: &Path, theme: &Path, out: &mut Vec<String>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for path in entries.flatten().map(|e| e.path()) {
        if path.is_dir() {
            if path != theme {
                walk(&path, theme, out);
            }
            continue;
        }
        if path.extension().is_some_and(|e| e == "slint") {
            check_file(&path, out);
        }
    }
}

fn check_file(path: &Path, out: &mut Vec<String>) {
    let text = fs::read_to_string(path).unwrap_or_default();
    for (n, line) in text.lines().enumerate() {
        for color in colors_in(line) {
            out.push(format!("{}:{}: {color}", path.display(), n + 1));
        }
        for literal in scale_literals(line) {
            out.push(format!("{}:{}: {literal}", path.display(), n + 1));
        }
    }
}

pub fn run() -> bool {
    let found = loose_colors(Path::new("crates/lamina-app/ui"));
    for f in &found {
        eprintln!("  cor fora do tema: {f} (use um token de ui/theme/)");
    }
    if found.is_empty() {
        println!("  cores ok");
    }
    found.is_empty()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn df_b1_cor_solta_numa_tela_reprova() {
        let dir = tempfile::tempdir().unwrap();
        let tela = dir.path().join("painel.slint");
        fs::write(&tela, "Rectangle {\n    background: #303030;\n}\n").unwrap();
        assert_eq!(
            loose_colors(dir.path()),
            [format!("{}:2: #303030", tela.display())]
        );
    }

    #[test]
    fn df_b2_cores_do_tema_e_transparent_passam() {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir_all(dir.path().join("theme")).unwrap();
        fs::write(dir.path().join("theme/palette.slint"), "x: #303030;\n").unwrap();
        let tela = "Rectangle { background: Theme.panel; border-color: transparent; }\n\
                    LineEdit { placeholder-text: \"#F5F0E6\"; } // #ffffff no comentário\n\
                    Image { colorize: Fixed.white-clear; }\n";
        fs::write(dir.path().join("painel.slint"), tela).unwrap();
        assert!(loose_colors(dir.path()).is_empty());
    }

    #[test]
    fn df_b15_letras_e_cantos_escritos_a_mao_reprovam() {
        assert_eq!(scale_literals("font-size: 12px;"), ["font-size: 12px"]);
        assert_eq!(
            scale_literals("x { border-radius: 4px; }"),
            ["border-radius: 4px"]
        );
        assert_eq!(scale_literals("font-weight: 700;"), ["font-weight: 700"]);
        assert_eq!(
            scale_literals("font-size: a ? 11px : 13px;"),
            ["font-size: a ? 11px : 13px"]
        );
        assert_eq!(
            scale_literals("font-weight: a ? 700 : 400;"),
            ["font-weight: a ? 700 : 400"]
        );
        assert!(scale_literals("font-size: Type.body; border-radius: self.height / 2;").is_empty());
    }

    #[test]
    fn cores_por_nome_e_funcao_tambem_reprovam() {
        assert_eq!(colors_in("color: white;"), ["white"]);
        assert_eq!(colors_in("color: red;"), ["red"]);
        assert!(colors_in("color: Fixed.red; x: red-line;").is_empty());
        assert_eq!(colors_in("x: rgba(0, 0, 0, 50%);"), ["rgba"]);
        assert!(colors_in("cor: Theme.text; # sozinho").is_empty());
    }
}
