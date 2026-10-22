//! Cenários da spec `rolo-de-laminas` (RL) que olham as telas: a árvore de acessibilidade enxuta
//! (RL-B28) e a linha do tempo que rola sem refazer as peças (RL-B29).

#![allow(clippy::unwrap_used, clippy::panic)]

use std::path::Path;

fn ui_files() -> Vec<(String, String)> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("ui");
    let mut files: Vec<(String, String)> = std::fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|x| x == "slint"))
        .map(|p| {
            let name = p.file_name().unwrap().to_string_lossy().into_owned();
            (name, std::fs::read_to_string(&p).unwrap())
        })
        .collect();
    files.sort();
    files
}

/// O começo do bloco que abre em `at` (o `{`): até o primeiro `{` ou `}` de dentro.
fn head_of_block(text: &str, at: usize) -> &str {
    let rest = &text[at + 1..];
    let end = rest.find(['{', '}']).unwrap_or(rest.len());
    &rest[..end]
}

/// O trecho do componente `name` (até o próximo `component` no começo da linha).
fn component<'a>(text: &'a str, name: &str) -> &'a str {
    let start = text
        .find(&format!("component {name} "))
        .unwrap_or_else(|| panic!("{name} não está no arquivo"));
    let rest = &text[start..];
    let end = rest[1..]
        .find("\ncomponent ")
        .or_else(|| rest[1..].find("\nexport component "))
        .map_or(rest.len(), |i| i + 1);
    &rest[..end]
}

#[test]
fn rl_b28_toda_imagem_fica_fora_da_arvore_de_acessibilidade() {
    let mut images = 0;
    for (file, text) in ui_files() {
        for word in ["Image {", "Tile {"] {
            for (at, _) in text.match_indices(word) {
                // `PhotoTile {` não é um `Tile {`: a palavra tem de começar ali (`SlotImage {` é
                // uma imagem e entra pelo "Image {").
                let prev = text[..at].chars().next_back();
                if word == "Tile {" && prev.is_some_and(|c| c.is_alphanumeric() || c == '_') {
                    continue;
                }
                let line_start = text[..at].rfind('\n').map_or(0, |i| i + 1);
                if text[line_start..at].trim_start().starts_with("//") {
                    continue;
                }
                let brace = at + word.len() - 1;
                let head = head_of_block(&text, brace);
                assert!(
                    head.contains("accessible-role: none"),
                    "{file}: `{word}` sem `accessible-role: none` logo no começo: {}",
                    &text[line_start..brace + 1]
                );
                images += 1;
            }
        }
    }
    assert!(images >= 20, "achou só {images} imagens");
}

#[test]
fn rl_b28_a_peca_da_grade_e_um_item_so_com_o_nome_da_foto() {
    let files = ui_files();
    let text = &files
        .iter()
        .find(|(f, _)| f == "photo_strip.slint")
        .unwrap()
        .1;
    let tile = component(text, "PhotoTile");
    assert!(tile.contains("accessible-role: list-item;"));
    assert!(tile.contains("accessible-label: root.data.uses > 0 ? root.data.name"));
    // O nome e o selo por dentro não viram nós próprios.
    for word in ["Text {", "badge := Text {"] {
        let at = tile.find(word).unwrap_or_else(|| panic!("{word} na peça"));
        let head = head_of_block(tile, at + word.len() - 1);
        assert!(head.contains("accessible-role: none"), "{word}");
    }
}

#[test]
fn rl_b29_a_peca_da_linha_do_tempo_nao_tem_partes_condicionais() {
    let files = ui_files();
    let text = &files.iter().find(|(f, _)| f == "timeline.slint").unwrap().1;
    let start = text.find("for t in root.timeline:").unwrap();
    let item = &text[start..];
    let end = item.find("\n        }\n").unwrap();
    let item = &item[..end];
    // `if (…)` dentro de um callback pode; `if cond: Elemento` não.
    let conditional = item
        .lines()
        .map(str::trim_start)
        .find(|l| l.starts_with("if ") && !l.starts_with("if ("));
    assert!(
        conditional.is_none(),
        "um `if` na peça refaz a peça quando a vaga troca de item: {conditional:?}"
    );
    assert!(item.contains("visible: !t.cut;") && item.contains("visible: t.cut;"));
}
