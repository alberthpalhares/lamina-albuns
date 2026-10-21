//! As novidades de cada versão (spec NV): o leitor do `novidades.md`, o mesmo no Lamina e na
//! release (`cargo xtask versao`), e quais versões a janela "O que há de novo" mostra. Nada aqui
//! lê a hora do computador (regra 10).

mod item;
mod page;

use item::{clean, item};
pub use page::{
    Illustration, NO_NEWS, NewsEntry, NewsPage, after_update, entries, from_help, heading,
    illustration, item_label, missing_try, next,
};

use super::Version;

/// Uma novidade: o título em negrito ("Atalhos do teclado."), o texto e o "Experimente".
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewsItem {
    pub title: Option<String>,
    pub text: String,
    pub try_it: Option<String>,
    /// A ilustração (`<!-- imagem: atalhos.png -->`), um arquivo de `dist/novidades/`.
    pub image: Option<String>,
    /// O comando do "Experimente agora" (`<!-- comando: ajuda.atalhos -->`).
    pub command: Option<String>,
}

/// A seção de uma versão: a data como está escrita, a frase de abertura e os itens.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewsSection {
    pub version: Version,
    pub date: Option<String>,
    pub intro: String,
    pub items: Vec<NewsItem>,
}

/// Lê as seções `## [versão] - data`, a mais nova primeiro (regra 2). Cabeçalho que não é versão
/// (`## [Não lançado]`) é pulado com tudo o que está embaixo; seção vazia não entra; duas seções
/// da mesma versão: vale a de cima.
pub fn parse(text: &str) -> Vec<NewsSection> {
    let mut sections: Vec<NewsSection> = Vec::new();
    let mut lines = text.lines().peekable();
    while let Some(line) = lines.next() {
        let Some((version, date)) = section_head(line) else {
            continue;
        };
        let mut body = Vec::new();
        while let Some(next) = lines.next_if(|l| !l.starts_with("## ")) {
            body.push(next);
        }
        let Some(version) = version else {
            continue;
        };
        let (intro, items) = body_parts(&body);
        let repeated = sections.iter().any(|s| s.version == version);
        if !repeated && (!intro.is_empty() || !items.is_empty()) {
            sections.push(NewsSection {
                version,
                date,
                intro,
                items,
            });
        }
    }
    sections.sort_by(|a, b| b.version.cmp(&a.version));
    sections
}

/// As seções que o fotógrafo ainda não viu: `previous` < versão ≤ `current`, a mais nova primeiro.
/// Numa versão estável, as de teste ficam de fora (regra 4).
pub fn since(sections: &[NewsSection], previous: &Version, current: &Version) -> Vec<NewsSection> {
    sections
        .iter()
        .filter(|s| s.version > *previous && s.version <= *current)
        .filter(|s| current.is_prerelease() || !s.version.is_prerelease())
        .cloned()
        .collect()
}

/// Todas as seções até a versão que roda, com as de teste (Ajuda → "O que há de novo…", regra 5).
pub fn until(sections: &[NewsSection], current: &Version) -> Vec<NewsSection> {
    sections
        .iter()
        .filter(|s| s.version <= *current)
        .cloned()
        .collect()
}

/// O texto da seção `## [versão]` como está no arquivo, para as notas da release (regra 8): as
/// linhas até o próximo `## `, sem os espaços das pontas. `None` se não há a seção ou ela é vazia.
pub fn section_text(text: &str, version: &str) -> Option<String> {
    let head = format!("## [{version}]");
    let start = text
        .lines()
        .position(|l| l.trim_start().starts_with(&head))?;
    let body: Vec<&str> = text
        .lines()
        .skip(start + 1)
        .take_while(|l| !l.starts_with("## "))
        .collect();
    let body = body.join("\n").trim().to_owned();
    (!body.is_empty()).then_some(body)
}

/// Os itens sem "Experimente" (a release avisa, regra 9).
pub fn without_try(section: &NewsSection) -> Vec<&NewsItem> {
    section
        .items
        .iter()
        .filter(|i| i.try_it.is_none())
        .collect()
}

/// A data como a janela mostra: "2026-10-24" vira "24/10/2026"; outro formato fica como está.
pub fn date_text(date: &str) -> String {
    let parts: Vec<&str> = date.split('-').collect();
    match parts[..] {
        [y, m, d]
            if [(y, 4), (m, 2), (d, 2)]
                .iter()
                .all(|(p, n)| p.len() == *n && p.chars().all(|c| c.is_ascii_digit())) =>
        {
            format!("{d}/{m}/{y}")
        }
        _ => date.to_owned(),
    }
}

/// `## [versão] - data`: a versão (`None` se o colchete não tem versão) e a data. `None` se a
/// linha não é um cabeçalho de seção.
fn section_head(line: &str) -> Option<(Option<Version>, Option<String>)> {
    let rest = line.strip_prefix("## ")?;
    let Some(inner) = rest.strip_prefix('[') else {
        return Some((None, None));
    };
    let (version, after) = inner.split_once(']')?;
    let date = after
        .trim()
        .strip_prefix('-')
        .map(|d| d.trim().to_owned())
        .filter(|d| !d.is_empty());
    Some((Version::parse(version), date))
}

/// A frase de abertura (o texto antes do primeiro item) e os itens. Linhas com recuo continuam o
/// item; um parágrafo solto depois dos itens vira um item sem título.
fn body_parts(body: &[&str]) -> (String, Vec<NewsItem>) {
    let mut intro: Vec<String> = Vec::new();
    let mut blocks: Vec<String> = Vec::new();
    let mut open = false;
    for line in body {
        // Subtítulo e citação viram texto simples, sem a marca do começo da linha.
        let trimmed = line.trim().trim_start_matches(['#', '>']).trim();
        if trimmed.is_empty() {
            open = false;
        } else if let Some(item) = trimmed
            .strip_prefix("- ")
            .filter(|_| !line.starts_with(' '))
        {
            blocks.push(item.to_owned());
            open = true;
        } else if open {
            if let Some(last) = blocks.last_mut() {
                last.push(' ');
                last.push_str(trimmed);
            }
        } else if blocks.is_empty() {
            intro.push(trimmed.to_owned());
        } else {
            blocks.push(trimmed.to_owned());
            open = true;
        }
    }
    (
        clean(&intro.join(" ")),
        blocks.iter().map(|b| item(b)).collect(),
    )
}
