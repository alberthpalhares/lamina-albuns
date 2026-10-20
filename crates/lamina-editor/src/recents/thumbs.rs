//! A miniatura do cartão e a conferência dos arquivos que sumiram (spec TI, regras 4 e 5). As duas
//! rodam fora da linha da janela: a janela chama e espera a resposta por mensagem.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use lamina_core::{PhotoId, Project};
use lamina_render::RenderTarget;

use crate::Cache;

/// 320 px no lado maior.
pub const THUMB_SIDE: u32 = 320;

/// Desenha a primeira lâmina com o renderizador da prévia (as miniaturas das fotos, do cache,
/// bastam nesse tamanho) e grava em `target`. `Err` com o motivo, em texto.
pub fn write_thumbnail(project: &Project, cache: &Cache, target: &Path) -> Result<(), String> {
    let thumbs: HashMap<PhotoId, PathBuf> = project
        .photos
        .iter()
        .filter_map(|p| cache.thumbnail(p).map(|path| (p.id, path)))
        .collect();
    let source = lamina_media::thumbnail_source(thumbs);
    // O renderizador acha o tamanho pela `preview_size`: 320 px no lado maior.
    let target_size = RenderTarget::Preview {
        max_side: THUMB_SIDE,
    };
    let image = lamina_render::render_spread(project, 0, target_size, &source)
        .map_err(|e| e.to_string())?;
    let folder = target.parent().unwrap_or(Path::new("."));
    std::fs::create_dir_all(folder).map_err(|e| e.to_string())?;
    let tmp = tempfile::Builder::new()
        .suffix(".png")
        .tempfile_in(folder)
        .map_err(|e| e.to_string())?;
    image
        .save_with_format(tmp.path(), image::ImageFormat::Png)
        .map_err(|e| e.to_string())?;
    tmp.persist(target).map_err(|e| e.error.to_string())?;
    Ok(())
}

/// Os caminhos que não existem mais. Só confere se o arquivo existe, sem ler (regra 4); pode
/// demorar num disco de rede, por isso a janela chama fora da linha dela.
pub fn missing_files(paths: &[PathBuf]) -> Vec<PathBuf> {
    paths.iter().filter(|p| !p.is_file()).cloned().collect()
}
