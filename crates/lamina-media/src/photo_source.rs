//! Fontes de fotos para a renderização (spec RD, seção 3): entregam os pixels de cada foto já
//! girados pelo EXIF. Moram aqui, e não na CLI, para a interface do Marco 2 usar as mesmas
//! (auditoria do Marco 1, M4).

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use image::RgbaImage;
use lamina_core::{Photo, PhotoId};

use crate::load_oriented;
use crate::thumbnail::THUMB_SIZE;

/// Resultado de uma fonte de fotos: a imagem compartilhada, ou o motivo em texto.
pub type Loaded = Result<Arc<RgbaImage>, String>;

/// Sempre o arquivo original: para a impressão.
pub fn original_source() -> impl Fn(&Photo, (u32, u32)) -> Loaded + Sync {
    |photo: &Photo, _: (u32, u32)| {
        load_oriented(&photo.path)
            .map(Arc::new)
            .map_err(|e| e.to_string())
    }
}

/// A miniatura do catálogo quando a foto inteira precisa de no máximo o tamanho dela (prévias);
/// senão, o arquivo original.
pub fn thumbnail_source(
    thumbs: HashMap<PhotoId, PathBuf>,
) -> impl Fn(&Photo, (u32, u32)) -> Loaded + Sync {
    move |photo: &Photo, needed: (u32, u32)| {
        if needed.0.max(needed.1) <= THUMB_SIZE
            && let Some(Ok(thumb)) = thumbs.get(&photo.id).map(image::open)
        {
            return Ok(Arc::new(thumb.into_rgba8()));
        }
        load_oriented(&photo.path)
            .map(Arc::new)
            .map_err(|e| e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use image::{Rgb, RgbImage};
    use lamina_core::{AlbumSpec, Dpi, Mm, Project};

    use super::*;

    /// Grava uma foto 60 × 40 e uma "miniatura" 30 × 20 de outra cor, e devolve o projeto.
    fn setup(dir: &std::path::Path) -> (Project, HashMap<PhotoId, PathBuf>) {
        let (photo, thumb) = (dir.join("foto.png"), dir.join("mini.png"));
        RgbImage::from_pixel(60, 40, Rgb([200, 0, 0]))
            .save(&photo)
            .unwrap();
        RgbImage::from_pixel(30, 20, Rgb([0, 0, 200]))
            .save(&thumb)
            .unwrap();
        let spec = AlbumSpec::new(Mm(300.0), Mm(300.0), Dpi::new(300).unwrap()).unwrap();
        let mut project = Project::new("teste", spec);
        let id = project.add_photo(&photo, 60, 40);
        (project, HashMap::from([(id, thumb)]))
    }

    #[test]
    fn miniatura_so_quando_basta() {
        let dir = tempfile::tempdir().unwrap();
        let (project, thumbs) = setup(dir.path());
        let photo = &project.photos[0];
        let source = thumbnail_source(thumbs);
        assert_eq!(
            source(photo, (100, 80)).unwrap().dimensions(),
            (30, 20),
            "miniatura"
        );
        let big = source(photo, (1000, 800)).unwrap();
        assert_eq!(big.dimensions(), (60, 40), "original");
    }

    #[test]
    fn original_sempre_le_o_arquivo() {
        let dir = tempfile::tempdir().unwrap();
        let (project, _) = setup(dir.path());
        let image = original_source()(&project.photos[0], (1, 1)).unwrap();
        assert_eq!(image.dimensions(), (60, 40));
    }

    #[test]
    fn arquivo_que_sumiu_vira_motivo_em_texto() {
        let spec = AlbumSpec::new(Mm(300.0), Mm(300.0), Dpi::new(300).unwrap()).unwrap();
        let mut project = Project::new("teste", spec);
        project.add_photo("nao-existe.jpg", 60, 40);
        let err = original_source()(&project.photos[0], (1, 1)).unwrap_err();
        assert!(err.contains("nao-existe.jpg"), "{err}");
    }
}
