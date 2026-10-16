//! `lamina auto --previa <pasta>`: grava cada lâmina como PNG com as fotos de verdade (spec RD,
//! seção 8). O trabalho fica na `lamina-export` (`write_previews`); aqui só a tradução.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use anyhow::Result;
use lamina_core::{PhotoId, Project};
use lamina_export::write_previews;
use lamina_media::thumbnail_source;

/// Maior lado das prévias gravadas, em pixels.
pub const PREVIEW_SIDE: u32 = 2000;

/// Grava as prévias em `dir` e devolve quantas foram gravadas.
pub fn write(project: &Project, thumbs: &HashMap<PhotoId, PathBuf>, dir: &Path) -> Result<usize> {
    let source = thumbnail_source(thumbs.clone());
    Ok(write_previews(project, dir, PREVIEW_SIDE, &source)?.len())
}
