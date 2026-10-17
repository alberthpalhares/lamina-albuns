//! Que arquivos cada lâmina gera (spec EX, regras 3 e 4): a lâmina inteira ou as páginas
//! soltas, com nomes ordenáveis.

use lamina_core::{PageSide, Project};

use crate::range::parse_range;
use crate::{ExportError, ExportFormat, ExportOptions, page_file_name, spread_file_name};

/// O que sai de uma lâmina.
pub enum Output {
    /// A lâmina inteira num arquivo.
    Whole(String),
    /// Uma página por arquivo (as inativas das pontas ficam de fora).
    Pages(Vec<(PageSide, String)>),
}

/// Uma lâmina a exportar (começando em 0) e o que sai dela.
pub struct SpreadJob {
    pub spread: usize,
    pub output: Output,
}

impl SpreadJob {
    /// O lado de cada arquivo de saída: nenhum para a lâmina inteira (spec PD, caixas).
    pub fn sides(&self) -> Vec<Option<PageSide>> {
        match &self.output {
            Output::Whole(_) => vec![None],
            Output::Pages(pages) => pages.iter().map(|(side, _)| Some(*side)).collect(),
        }
    }

    pub fn names(&self) -> Vec<&str> {
        match &self.output {
            Output::Whole(name) => vec![name.as_str()],
            Output::Pages(pages) => pages.iter().map(|(_, n)| n.as_str()).collect(),
        }
    }
}

/// Lâminas pedidas e os arquivos de cada uma, já validados.
pub fn plan(project: &Project, options: &ExportOptions) -> Result<Vec<SpreadJob>, ExportError> {
    if !(1..=100).contains(&options.quality) {
        return Err(ExportError::Quality(options.quality));
    }
    match options.format {
        ExportFormat::Jpeg => {
            if options.crop_marks {
                return Err(ExportError::CropMarksNeedPdf);
            }
        }
        ExportFormat::Pdf => {}
        ExportFormat::Tiff if options.crop_marks => return Err(ExportError::CropMarksNeedPdf),
        ExportFormat::Tiff => {}
        other => return Err(ExportError::Format(other.extension())),
    }
    let total = project.spreads.len();
    if total == 0 {
        return Err(ExportError::NoSpreads);
    }
    let spreads = match &options.range {
        Some(text) => parse_range(text, total)?,
        None => (0..total).collect(),
    };
    // JPEG ou TIFF: o PDF não usa estes nomes, só a lista de lâminas e páginas.
    let format = match options.format {
        ExportFormat::Tiff => ExportFormat::Tiff,
        _ => ExportFormat::Jpeg,
    };
    let pages = options.pages.then(|| page_names(project, format));
    Ok(spreads
        .into_iter()
        .map(|spread| {
            let output = match &pages {
                Some(all) => Output::Pages(all[spread].clone()),
                None => Output::Whole(spread_file_name(spread + 1, total, format)),
            };
            SpreadJob { spread, output }
        })
        .collect())
}

/// Nome de cada página ativa, lâmina por lâmina, numeradas em sequência no álbum inteiro.
fn page_names(project: &Project, format: ExportFormat) -> Vec<Vec<(PageSide, String)>> {
    let edges = project.spec.edges();
    let total = project.spreads.len();
    let pages = edges.page_count(total);
    let mut number = 0;
    (0..total)
        .map(|s| {
            let active = edges.active_pages(s, total);
            [
                (active.left, PageSide::Left),
                (active.right, PageSide::Right),
            ]
            .into_iter()
            .filter(|(on, _)| *on)
            .map(|(_, side)| {
                number += 1;
                (side, page_file_name(number, pages, format))
            })
            .collect()
        })
        .collect()
}
