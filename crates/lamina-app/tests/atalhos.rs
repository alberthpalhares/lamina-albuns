//! A trava dos rótulos do menu (spec AT, regra 9; AT-B9): o menu escreve o atalho à mão
//! ("Salvar  (Ctrl+S)"), e esta trava confere que ele diz a mesma tecla da tabela de comandos.
//! Lê todos os `.slint` da pasta `ui`, para continuar valendo quando o `app.slint` for dividido.

#![allow(clippy::unwrap_used, clippy::panic)]

use std::path::Path;

use lamina_editor::Command;

/// Um item de menu com `root.command("…")`: o nome do comando e os rótulos do título.
#[derive(Debug)]
struct Item {
    command: String,
    labels: Vec<String>,
}

/// Os textos entre aspas de `text`.
fn strings(text: &str) -> Vec<String> {
    text.split('"')
        .skip(1)
        .step_by(2)
        .map(str::to_owned)
        .collect()
}

/// Os nomes de `command("…")` em `text`, de qualquer componente (menu, botões, paleta).
fn command_names(text: &str) -> Vec<String> {
    text.split("command(\"")
        .skip(1)
        .filter_map(|rest| rest.split('"').next())
        .map(str::to_owned)
        .collect()
}

/// O trecho de um `MenuItem { … }` até a chave que fecha (fora das aspas).
fn item_text(text: &str) -> &str {
    let (mut depth, mut quoted) = (0, false);
    for (i, c) in text.char_indices() {
        match c {
            '"' => quoted = !quoted,
            '{' if !quoted => depth += 1,
            '}' if !quoted => {
                depth -= 1;
                if depth == 0 {
                    return &text[..=i];
                }
            }
            _ => {}
        }
    }
    text
}

/// Os itens de menu de um arquivo `.slint` que rodam um comando do registro.
fn menu_items(text: &str) -> Vec<Item> {
    text.match_indices("MenuItem {")
        .filter_map(|(at, _)| {
            let item = item_text(&text[at..]);
            let command = item.split("root.command(\"").nth(1)?.split('"').next()?;
            let title = item.split("title:").nth(1)?.split(';').next()?;
            let labels = strings(title)
                .into_iter()
                .filter(|s| s.chars().any(char::is_alphabetic))
                .collect();
            Some(Item {
                command: command.to_owned(),
                labels,
            })
        })
        .collect()
}

/// O atalho que o rótulo mostra: o que vem no fim, entre parênteses, depois de dois espaços.
fn shown_key(label: &str) -> Option<(&str, &str)> {
    let (name, rest) = label.rsplit_once("  (")?;
    Some((name, rest.strip_suffix(')')?))
}

/// Os problemas dos itens: comando que não existe e rótulo com a tecla diferente da tabela.
fn problems(items: &[Item]) -> Vec<String> {
    let mut out = Vec::new();
    for item in items {
        let Some(command) = Command::from_name(&item.command) else {
            out.push(format!("{}: o comando não existe", item.command));
            continue;
        };
        let table = command.shortcut().map(|s| s.text());
        for label in &item.labels {
            let shown = shown_key(label);
            let name = shown.map_or(label.as_str(), |(name, _)| name);
            match (shown.map(|(_, key)| key), table.as_deref()) {
                (Some(menu), Some(table)) if menu != table => {
                    out.push(format!("{name}: o menu diz {menu}, a tabela diz {table}"));
                }
                (Some(menu), None) => {
                    out.push(format!(
                        "{name}: o menu diz {menu}, a tabela não tem atalho"
                    ));
                }
                (None, Some(table)) => {
                    out.push(format!(
                        "{name}: o menu não diz o atalho, a tabela diz {table}"
                    ));
                }
                _ => {}
            }
        }
    }
    out
}

#[test]
fn at_b9_os_rotulos_do_menu_dizem_a_tecla_da_tabela() {
    let ui = Path::new(env!("CARGO_MANIFEST_DIR")).join("ui");
    let (mut items, mut names) = (Vec::new(), Vec::new());
    for entry in std::fs::read_dir(&ui).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().is_some_and(|e| e == "slint") {
            let text = std::fs::read_to_string(&path).unwrap();
            items.extend(menu_items(&text));
            names.extend(command_names(&text));
        }
    }
    assert!(
        items.len() >= 30,
        "só {} itens de menu: a trava não achou o menu",
        items.len()
    );
    for name in &names {
        assert!(
            Command::from_name(name).is_some(),
            "{name}: o comando não existe"
        );
    }
    let found = problems(&items);
    assert!(found.is_empty(), "{}", found.join("\n"));
    // Os dois lados de `guias ? "Esconder…" : "Mostrar…"` contam, e o ✓ sem letras não.
    let guides = items.iter().find(|i| i.command == "ver.guias").unwrap();
    assert_eq!(guides.labels, ["Esconder guias  (G)", "Mostrar guias  (G)"]);
    let strip = items
        .iter()
        .find(|i| i.command == "ver.faixa-laminas")
        .unwrap();
    assert_eq!(strip.labels, ["Faixa de lâminas  (Alt+1)"]);
}

#[test]
fn at_b9_tecla_errada_ou_comando_inexistente_reprovam() {
    let menu = r#"
        MenuItem { title: "Salvar  (Ctrl+D)"; activated => { root.command("arquivo.salvar"); } }
        MenuItem { title: "Presets…  (Ctrl+P)"; activated => { root.command("arquivo.presets"); } }
        MenuItem { title: "Exportar…"; activated => { root.command("arquivo.exportar"); } }
        MenuItem { title: "Receber versões de teste (beta)"; activated => { root.command("ajuda.versoes-de-teste"); } }
        MenuItem { title: "Nada"; activated => { root.command("arquivo.nada"); } }
        MenuItem { title: item.label; activated => { RecentsUi.open(item.index); } }
    "#;
    let items = menu_items(menu);
    assert_eq!(items.len(), 5, "o item sem root.command fica de fora");
    assert_eq!(
        problems(&items),
        [
            "Salvar: o menu diz Ctrl+D, a tabela diz Ctrl+S",
            "Presets…: o menu diz Ctrl+P, a tabela não tem atalho",
            "Exportar…: o menu não diz o atalho, a tabela diz Ctrl+E",
            "arquivo.nada: o comando não existe",
        ]
    );
    assert_eq!(
        command_names(menu),
        [
            "arquivo.salvar",
            "arquivo.presets",
            "arquivo.exportar",
            "ajuda.versoes-de-teste",
            "arquivo.nada"
        ]
    );
}
