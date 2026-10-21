//! Um item das novidades (spec NV, regra 2): o título, o texto, o "Experimente" e, num comentário
//! que a página da versão não mostra, a ilustração e o comando do "Experimente agora".

use super::NewsItem;

/// Um item: o `**título**` do começo, o texto e o que vem depois de "Experimente:".
pub(super) fn item(block: &str) -> NewsItem {
    let (block, image, command) = extras(block);
    let block = block.as_str();
    let (title, rest) = match block.strip_prefix("**").and_then(|b| b.split_once("**")) {
        Some((title, rest)) => (Some(clean(title)), rest),
        None => (None, block),
    };
    let rest = clean(rest);
    let (text, try_it) = match rest.split_once("Experimente:") {
        Some((text, try_it)) => (text.trim().to_owned(), Some(try_it.trim().to_owned())),
        None => (rest, None),
    };
    NewsItem {
        title,
        text,
        try_it: try_it.filter(|t| !t.is_empty()),
        image,
        command,
    }
}

/// Tira do item o comentário `<!-- imagem: x.png; comando: y -->`, que a página da versão não
/// mostra, e devolve a imagem e o comando dele.
fn extras(block: &str) -> (String, Option<String>, Option<String>) {
    let Some((before, rest)) = block.split_once("<!--") else {
        return (block.to_owned(), None, None);
    };
    let (inside, after) = rest.split_once("-->").unwrap_or((rest, ""));
    let field = |key: &str| {
        inside
            .split(';')
            .filter_map(|pair| pair.split_once(':'))
            .find(|(k, _)| k.trim() == key)
            .map(|(_, v)| v.trim().to_owned())
            .filter(|v| !v.is_empty())
    };
    let text = format!("{} {}", before.trim_end(), after.trim_start());
    (text.trim().to_owned(), field("imagem"), field("comando"))
}

/// Tira as marcas do Markdown: `**`, `*`, `_`, as crases e o endereço de `[texto](endereço)`.
pub(super) fn clean(text: &str) -> String {
    let mut out = String::new();
    let mut rest = text;
    while let Some(open) = rest.find('[') {
        let link = rest[open + 1..]
            .split_once("](")
            .and_then(|(label, after)| Some((label, after.split_once(')')?.1)));
        out.push_str(&rest[..open]);
        match link {
            Some((label, after)) => {
                out.push_str(label);
                rest = after;
            }
            None => {
                out.push('[');
                rest = &rest[open + 1..];
            }
        }
    }
    out.push_str(rest);
    out.retain(|c| !matches!(c, '*' | '_' | '`'));
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}
