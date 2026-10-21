//! A busca da lista de atalhos (spec AT, regra 6): sem acento e sem maiúscula, pelo que a linha
//! faz, pelo grupo, pelo gesto ou pela tecla ("ctrl+s", "⌘s", "seta").

use super::{Group, Row};

/// A dica embaixo de "Nenhum atalho com …" (seção 6).
pub const NO_MATCH_HINT: &str = "Tente uma palavra, como “lâmina”, ou uma tecla, como “ctrl+z”.";

/// A frase da busca sem resultado: "Nenhum atalho com “xyz”.".
pub fn no_match(query: &str) -> String {
    format!("Nenhum atalho com “{}”.", query.trim())
}

/// As linhas que atendem a `query`, nos grupos e na ordem da lista; grupo vazio some. Busca vazia
/// ou só com espaços devolve tudo.
pub fn search(groups: &[Group], query: &str) -> Vec<Group> {
    let words = fold(query.trim());
    if words.is_empty() {
        return groups.to_vec();
    }
    let key = parse_key(&words);
    groups
        .iter()
        .filter_map(|g| {
            let in_title = fold(g.title).contains(&words);
            let rows: Vec<Row> = g
                .rows
                .iter()
                .filter(|r| in_title || row_matches(r, &words, key.as_ref()))
                .cloned()
                .collect();
            (!rows.is_empty()).then_some(Group {
                title: g.title,
                rows,
            })
        })
        .collect()
}

/// Minúsculas e sem acentos, para as buscas (também a da tela inicial, spec TI, regra 8).
pub fn fold(text: &str) -> String {
    text.chars()
        .flat_map(char::to_lowercase)
        .map(|c| match c {
            'á' | 'à' | 'â' | 'ã' | 'ä' => 'a',
            'é' | 'è' | 'ê' | 'ë' => 'e',
            'í' | 'ì' | 'î' | 'ï' => 'i',
            'ó' | 'ò' | 'ô' | 'õ' | 'ö' => 'o',
            'ú' | 'ù' | 'û' | 'ü' => 'u',
            'ç' => 'c',
            'ñ' => 'n',
            other => other,
        })
        .collect()
}

fn row_matches(row: &Row, words: &str, key: Option<&KeyText>) -> bool {
    if fold(row.does).contains(words) {
        return true;
    }
    // O "como" das linhas fixas também é texto ("arrastar", "Alt+setas").
    if row.command.is_none() && row.keys.iter().any(|k| fold(k).contains(words)) {
        return true;
    }
    row.keys.iter().any(|k| {
        let mut k = fold(k);
        k.retain(|c| !c.is_whitespace());
        named_by(&k, words) || (key.is_some() && parse_key(&k).as_ref() == key)
    })
}

/// As setas também atendem por "seta" e a barra por "barra".
fn named_by(key: &str, words: &str) -> bool {
    match words {
        "seta" | "setas" => key.contains(['←', '→', '↑', '↓']),
        "barra" => key.contains('/'),
        _ => false,
    }
}

/// Uma tecla escrita de qualquer jeito, sem os espaços: Ctrl (ou Cmd, ⌘), Alt (Option, ⌥), Shift
/// (⇧) e o nome, para "ctrl+shift+s", "shift+ctrl+s" e "⇧⌘s" serem a mesma tecla.
#[derive(Debug, PartialEq, Eq)]
struct KeyText {
    ctrl: bool,
    alt: bool,
    shift: bool,
    name: String,
}

const CTRL: [&str; 5] = ["ctrl+", "control+", "cmd+", "command+", "⌘"];
const ALT: [&str; 4] = ["alt+", "option+", "opt+", "⌥"];
const SHIFT: [&str; 2] = ["shift+", "⇧"];

fn parse_key(text: &str) -> Option<KeyText> {
    let mut rest: String = text.chars().filter(|c| !c.is_whitespace()).collect();
    let mut key = KeyText {
        ctrl: false,
        alt: false,
        shift: false,
        name: String::new(),
    };
    loop {
        let before = rest.len();
        key.ctrl |= strip(&mut rest, &CTRL);
        key.alt |= strip(&mut rest, &ALT);
        key.shift |= strip(&mut rest, &SHIFT);
        if rest.len() == before {
            break;
        }
    }
    key.name = match rest.as_str() {
        "⌦" | "del" => "delete".into(),
        "⌫" => "backspace".into(),
        "↩" | "return" => "enter".into(),
        "escape" => "esc".into(),
        _ => rest,
    };
    (!key.name.is_empty()).then_some(key)
}

/// Tira do começo de `text` um dos prefixos (e o "+" que vier logo depois de um símbolo).
fn strip(text: &mut String, prefixes: &[&str]) -> bool {
    let Some(p) = prefixes
        .iter()
        .find(|p| text.starts_with(**p) && text.len() > p.len())
    else {
        return false;
    };
    text.replace_range(..p.len(), "");
    if !p.ends_with('+') && text.len() > 1 && text.starts_with('+') {
        text.replace_range(..1, "");
    }
    true
}
