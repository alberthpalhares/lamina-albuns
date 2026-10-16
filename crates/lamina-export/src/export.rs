//! Exportação (spec `exportacao`): um JPEG por lâmina ou por página, gravado com segurança.

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use lamina_core::Project;
use lamina_render::PhotoSource;
use rayon::prelude::*;

use crate::ExportError;
use crate::jobs::{SpreadJob, plan};
use crate::preflight::{Warning, preflight};
use crate::write::{self, Ctx};

/// Máximo de lâminas de impressão processadas ao mesmo tempo (~175 MB cada entre a imagem e
/// a cópia em RGB; regra 7). Codificar o JPEG é o passo mais lento e usa um núcleo por lâmina.
const AT_ONCE: usize = 4;

/// Lâminas ao mesmo tempo: até `AT_ONCE`, nunca mais que os núcleos da máquina.
fn at_once() -> usize {
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
    /// Substituir arquivos que já existem.
    pub overwrite: bool,
}

impl Default for ExportOptions {
    fn default() -> Self {
        ExportOptions {
            quality: DEFAULT_QUALITY,
            range: None,
            pages: false,
            overwrite: false,
        }
    }
}

/// O que foi gravado e os avisos da pré-impressão.
#[derive(Debug, Clone, PartialEq)]
pub struct ExportReport {
    /// Arquivos gravados, na ordem do álbum.
    pub files: Vec<PathBuf>,
    pub warnings: Vec<Warning>,
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
    let jobs = plan(project, options)?;
    if !options.overwrite {
        refuse_existing(&jobs, folder)?;
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
    };
    let files = write_all(&ctx, &jobs, progress)?;
    let spreads: Vec<usize> = jobs.iter().map(|j| j.spread).collect();
    Ok(ExportReport {
        files,
        warnings: preflight(project, &spreads),
        elapsed: started.elapsed(),
    })
}

/// Nomes dos arquivos que a exportação vai gravar, na ordem do álbum, sem gravar nada (para
/// quem precisa conferir a pasta antes, como o `lamina build`).
pub fn output_names(
    project: &Project,
    options: &ExportOptions,
) -> Result<Vec<String>, ExportError> {
    Ok(plan(project, options)?
        .iter()
        .flat_map(|j| j.names().into_iter().map(str::to_owned).collect::<Vec<_>>())
        .collect())
}

/// Nada é gravado se algum arquivo de saída já existe (regra 5).
fn refuse_existing(jobs: &[SpreadJob], folder: &Path) -> Result<(), ExportError> {
    let existing = jobs
        .iter()
        .flat_map(SpreadJob::names)
        .find(|name| folder.join(name).exists());
    match existing {
        Some(name) => Err(ExportError::Exists {
            name: name.to_owned(),
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
