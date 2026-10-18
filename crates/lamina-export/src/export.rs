//! Exportação (spec `exportacao`): um JPEG por lâmina ou por página, gravado com segurança.

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use std::borrow::Cow;

use lamina_core::{ColorSpace, FrameContent, Project};
use lamina_render::PhotoSource;
use rayon::prelude::*;

use crate::clean::remove_leftovers;
use crate::jobs::{SpreadJob, plan};
use crate::preflight::{Warning, preflight};
use crate::write::{self, Ctx};
use crate::{ExportError, ExportFormat, pdf};

/// Máximo de lâminas de impressão processadas ao mesmo tempo (~175 MB cada entre a imagem e
/// a cópia em RGB; regra 7). Codificar o JPEG é o passo mais lento e usa um núcleo por lâmina.
const AT_ONCE: usize = 4;

/// Lâminas ao mesmo tempo: até `AT_ONCE`, nunca mais que os núcleos da máquina.
pub(crate) fn at_once() -> usize {
    std::thread::available_parallelism().map_or(1, |n| n.get().min(AT_ONCE))
}

/// Qualidade padrão do JPEG: 95, com cor em resolução total (4:4:4) (AGENTS.md, regra 14;
/// decisão do dono em 2026-10-15). Imperceptível em fotos (~48 dB numa foto real) com ~35% menos
/// espaço que a 100.
pub const DEFAULT_QUALITY: u8 = 95;

/// Opções da exportação; os padrões vêm da spec.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExportOptions {
    /// Qualidade do JPEG, de 1 a 100.
    pub quality: u8,
    /// Intervalo de lâminas (ex.: "1, 3-4"); `None` = todas.
    pub range: Option<String>,
    /// Um arquivo por página em vez de um por lâmina.
    pub pages: bool,
    /// Substituir arquivos que já existem e apagar os que sobraram de uma exportação anterior
    /// (spec EI, regra 4).
    pub overwrite: bool,
    /// JPEG (um arquivo por lâmina ou página) ou PDF (um arquivo do álbum; spec PD).
    pub format: ExportFormat,
    /// Marcas de corte, só no PDF (spec PD, regra 6).
    pub crop_marks: bool,
    /// Espaço de cor da saída (spec CT). A fonte de fotos passada a [`export`] tem que entregar as
    /// fotos já nele (`lamina_media::original_source_in`).
    pub space: ColorSpace,
}

impl Default for ExportOptions {
    fn default() -> Self {
        ExportOptions {
            quality: DEFAULT_QUALITY,
            range: None,
            pages: false,
            overwrite: false,
            format: ExportFormat::Jpeg,
            crop_marks: false,
            space: ColorSpace::Srgb,
        }
    }
}

/// O que foi gravado e os avisos da pré-impressão.
#[derive(Debug, Clone, PartialEq)]
pub struct ExportReport {
    /// Arquivos gravados, na ordem do álbum.
    pub files: Vec<PathBuf>,
    pub warnings: Vec<Warning>,
    /// Arquivos de uma exportação anterior apagados ao substituir (spec EI, regra 4).
    pub removed: Vec<PathBuf>,
    pub elapsed: Duration,
}

/// Exporta o projeto para `folder`. `progress` recebe (feitos, total) a cada arquivo gravado.
pub fn export(
    project: &Project,
    folder: &Path,
    photos: &dyn PhotoSource,
    options: &ExportOptions,
    progress: &dyn Fn(usize, usize),
) -> Result<ExportReport, ExportError> {
    let started = Instant::now();
    let in_space = colors_in(project, options.space);
    let project = in_space.as_ref();
    let jobs = plan(project, options)?;
    let names = names_of(project, options, &jobs);
    if !options.overwrite {
        refuse_existing(&names, folder)?;
    }
    std::fs::create_dir_all(folder).map_err(|e| ExportError::Folder {
        folder: folder.display().to_string(),
        reason: e.to_string(),
    })?;
    let ctx = Ctx {
        project,
        folder,
        photos,
        quality: options.quality,
        space: options.space,
        format: options.format,
    };
    let files = match options.format {
        ExportFormat::Pdf => vec![pdf::write(&ctx, &jobs, options, progress)?],
        _ => write_all(&ctx, &jobs, progress)?,
    };
    // A limpeza só vale para os JPEGs e TIFFs (spec PD, regra 7; spec CT, regra 6).
    let removed = if options.overwrite && options.format != ExportFormat::Pdf {
        remove_leftovers(folder, &names)?
    } else {
        Vec::new()
    };
    let spreads: Vec<usize> = jobs.iter().map(|j| j.spread).collect();
    Ok(ExportReport {
        files,
        warnings: preflight(project, &spreads),
        removed,
        elapsed: started.elapsed(),
    })
}

/// O projeto com fundo e borda no espaço de cor da saída (spec CT, regra 3). Em sRGB, o próprio
/// projeto, sem cópia (nada muda: regra 7).
fn colors_in(project: &Project, space: ColorSpace) -> Cow<'_, Project> {
    if space == ColorSpace::Srgb {
        return Cow::Borrowed(project);
    }
    let mut copy = project.clone();
    for spread in &mut copy.spreads {
        spread.background = space.from_srgb(spread.background);
        spread.background_right = spread.background_right.map(|c| space.from_srgb(c));
        for frame in &mut spread.frames {
            frame.border.color = space.from_srgb(frame.border.color);
            if let FrameContent::Text(text) = &mut frame.content {
                text.color = space.from_srgb(text.color);
            }
        }
    }
    Cow::Owned(copy)
}

/// Nomes dos arquivos que a exportação vai gravar, na ordem do álbum, sem gravar nada (para
/// quem precisa conferir a pasta antes, como o `lamina build`).
pub fn output_names(
    project: &Project,
    options: &ExportOptions,
) -> Result<Vec<String>, ExportError> {
    let jobs = plan(project, options)?;
    Ok(names_of(project, options, &jobs))
}

/// Os arquivos de saída: os JPEGs de cada lâmina, ou o PDF do álbum.
fn names_of(project: &Project, options: &ExportOptions, jobs: &[SpreadJob]) -> Vec<String> {
    match options.format {
        ExportFormat::Pdf => vec![pdf::file_name(project)],
        _ => jobs
            .iter()
            .flat_map(|j| j.names().into_iter().map(str::to_owned).collect::<Vec<_>>())
            .collect(),
    }
}

/// Nada é gravado se algum arquivo de saída já existe (regra 5).
fn refuse_existing(names: &[String], folder: &Path) -> Result<(), ExportError> {
    match names.iter().find(|name| folder.join(name).exists()) {
        Some(name) => Err(ExportError::Exists {
            name: name.clone(),
            folder: folder.display().to_string(),
        }),
        None => Ok(()),
    }
}

/// Grava em lotes de até `at_once()` lâminas. Se uma falha, o erro leva os arquivos que ficaram
/// gravados (inclusive os das outras lâminas do mesmo lote).
fn write_all(
    ctx: &Ctx,
    jobs: &[SpreadJob],
    progress: &dyn Fn(usize, usize),
) -> Result<Vec<PathBuf>, ExportError> {
    let total: usize = jobs.iter().map(|j| j.names().len()).sum();
    let mut files = Vec::with_capacity(total);
    for chunk in jobs.chunks(at_once()) {
        let written: Vec<Result<Vec<PathBuf>, ExportError>> = chunk
            .par_iter()
            .map(|job| write::spread(ctx, job))
            .collect();
        let mut failure = None;
        for result in written {
            match result {
                Ok(paths) => {
                    for path in paths {
                        files.push(path);
                        progress(files.len(), total);
                    }
                }
                Err(e) => failure = failure.or(Some(e)),
            }
        }
        if let Some(source) = failure {
            return Err(ExportError::Stopped {
                done: files,
                source: Box::new(source),
            });
        }
    }
    Ok(files)
}
