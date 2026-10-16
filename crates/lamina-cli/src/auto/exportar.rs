//! `lamina auto --exportar <pasta>`: grava os JPEGs de impressão (spec EX, seção 8) e mostra a
//! pré-impressão antes. O comando final é `lamina build` (pedaço CB).

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use anyhow::Result;
use lamina_core::{PhotoId, Project};
use lamina_export::{ExportOptions, parse_range, preflight};

use lamina_media::thumbnail_source;

use crate::relatorio::{export_with_progress, print_warnings};

/// Mostra a pré-impressão e exporta, mostrando o andamento.
pub fn run(
    project: &Project,
    thumbs: &HashMap<PhotoId, PathBuf>,
    folder: &Path,
    options: &ExportOptions,
) -> Result<()> {
    // Pré-impressão antes de exportar: se a exportação falhar, os avisos já foram vistos.
    let total = project.spreads.len();
    let spreads = match &options.range {
        Some(text) => parse_range(text, total)?,
        None => (0..total).collect(),
    };
    print_warnings(&preflight(project, &spreads));
    let source = thumbnail_source(thumbs.clone());
    export_with_progress(project, folder, &source, options)?;
    Ok(())
}
