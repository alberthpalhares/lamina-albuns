//! Prévias das lâminas em PNG (spec RD, seção 8): o mesmo renderizador da impressão, em tamanho
//! de tela. Mora aqui, e não na CLI, para a interface do Marco 2 usar o mesmo código (auditoria
//! do Marco 1, M4).

use std::path::{Path, PathBuf};

use lamina_core::Project;
use lamina_render::{PhotoSource, RenderTarget, render_spread};
use rayon::prelude::*;

use crate::{ExportError, ExportFormat, spread_file_name};

/// Grava `lamina-01.png`, `lamina-02.png`… (maior lado `max_side`) em `folder`. Cada lâmina é
/// gravada assim que fica pronta, sem guardar todas na memória; o erro devolvido é o da primeira
/// lâmina com problema.
pub fn write_previews(
    project: &Project,
    folder: &Path,
    max_side: u32,
    photos: &dyn PhotoSource,
) -> Result<Vec<PathBuf>, ExportError> {
    std::fs::create_dir_all(folder).map_err(|e| ExportError::Folder {
        folder: folder.display().to_string(),
        reason: e.to_string(),
    })?;
    let total = project.spreads.len();
    let target = RenderTarget::Preview { max_side };
    let results: Vec<Result<PathBuf, ExportError>> = (0..total)
        .into_par_iter()
        .map(|i| {
            let image = render_spread(project, i, target, photos)?;
            let path = folder.join(spread_file_name(i + 1, total, ExportFormat::Png));
            image.save(&path).map_err(|e| ExportError::Write {
                path: path.display().to_string(),
                reason: e.to_string(),
            })?;
            Ok(path)
        })
        .collect();
    results.into_iter().collect()
}

/// A prévia de uma lâmina (`index` a partir de 0) em PNG, na memória (maior lado `max_side`): a
/// imagem que o controle por IA mostra (spec IA, seção 4).
pub fn preview_png(
    project: &Project,
    index: usize,
    max_side: u32,
    photos: &dyn PhotoSource,
) -> Result<Vec<u8>, ExportError> {
    let image = render_spread(project, index, RenderTarget::Preview { max_side }, photos)?;
    let mut bytes = std::io::Cursor::new(Vec::new());
    image
        .write_to(&mut bytes, image::ImageFormat::Png)
        .map_err(|e| ExportError::Encode(e.to_string()))?;
    Ok(bytes.into_inner())
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use image::RgbaImage;
    use lamina_core::{AlbumSpec, Dpi, Mm, Photo};

    use super::*;

    #[test]
    fn grava_uma_previa_por_lamina() {
        let dir = tempfile::tempdir().unwrap();
        let spec =
            AlbumSpec::new(Mm::from_cm(30.0), Mm::from_cm(30.0), Dpi::new(300).unwrap()).unwrap();
        let mut project = Project::new("teste", spec);
        project.add_spread();
        project.add_spread();
        let source = |_: &Photo, _: (u32, u32)| Err::<Arc<RgbaImage>, String>("sem fotos".into());
        let files = write_previews(&project, dir.path(), 2000, &source).unwrap();
        assert_eq!(files.len(), 2);
        assert!(files[0].ends_with("lamina-01.png") && files[1].ends_with("lamina-02.png"));
        let first = image::open(&files[0]).unwrap();
        assert_eq!((first.width(), first.height()), (2000, 1000));
    }

    #[test]
    fn previa_em_png_na_memoria_tem_o_tamanho_pedido() {
        let spec =
            AlbumSpec::new(Mm::from_cm(30.0), Mm::from_cm(30.0), Dpi::new(300).unwrap()).unwrap();
        let mut project = Project::new("teste", spec);
        project.add_spread();
        let source = |_: &Photo, _: (u32, u32)| Err::<Arc<RgbaImage>, String>("sem fotos".into());
        let bytes = preview_png(&project, 0, 1024, &source).unwrap();
        let image = image::load_from_memory_with_format(&bytes, image::ImageFormat::Png).unwrap();
        assert_eq!((image.width(), image.height()), (1024, 512));
        assert!(matches!(
            preview_png(&project, 3, 1024, &source),
            Err(ExportError::Render(lamina_render::RenderError::NoSpread {
                number: 4,
                count: 1
            }))
        ));
    }
}
