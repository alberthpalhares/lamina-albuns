//! As imagens já convertidas para o Slint: miniaturas (faixa e navegador) e imagens de tela (lâmina
//! grande). Na memória ficam só as imagens de tela da lâmina atual (spec JP, regra 6).

use std::collections::HashMap;
use std::sync::Arc;

use lamina_core::{FrameContent, PhotoId, Project};
use lamina_editor::screens::Rgba;
use lamina_editor::{Cache, ScreenImages};
use slint::{Image, Rgba8Pixel, SharedPixelBuffer};

#[derive(Default)]
pub struct Images {
    screens: ScreenImages,
    converted: HashMap<PhotoId, (Arc<Rgba>, Image)>,
    thumbs: HashMap<PhotoId, Image>,
    /// A foto original do quadro em modo recorte (spec RC, regra 9).
    original: Option<(PhotoId, Image)>,
}

impl Images {
    /// Esquece tudo (projeto novo ou imagens recém-geradas no cache).
    pub fn clear(&mut self) {
        *self = Images::default();
    }

    /// A miniatura da foto, se estiver no cache.
    pub fn thumb(&mut self, project: &Project, id: PhotoId, cache: &Cache) -> Option<Image> {
        if let Some(img) = self.thumbs.get(&id) {
            return Some(img.clone());
        }
        let photo = project.photo(id).ok()?;
        let img = Image::load_from_path(&cache.thumbnail(photo)?).ok()?;
        self.thumbs.insert(id, img.clone());
        Some(img)
    }

    /// As miniaturas de todas as fotos do projeto.
    pub fn all_thumbs(&mut self, project: &Project, cache: &Cache) -> HashMap<PhotoId, Image> {
        project
            .photos
            .iter()
            .filter_map(|p| Some((p.id, self.thumb(project, p.id, cache)?)))
            .collect()
    }

    /// As imagens das fotos da lâmina `index`: a de tela, ou a miniatura enquanto ela não existe.
    pub fn spread(
        &mut self,
        project: &Project,
        index: usize,
        cache: &Cache,
    ) -> HashMap<PhotoId, Image> {
        self.screens.keep_around(project, index, cache);
        let mut images = HashMap::new();
        for id in photos_of(project, index) {
            let original = self
                .original
                .as_ref()
                .filter(|(o, _)| *o == id)
                .map(|(_, img)| img.clone());
            let img = original
                .or_else(|| self.screen(id))
                .or_else(|| self.thumb(project, id, cache));
            if let Some(img) = img {
                images.insert(id, img);
            }
        }
        self.converted.retain(|id, _| images.contains_key(id));
        images
    }

    pub fn original_id(&self) -> Option<PhotoId> {
        self.original.as_ref().map(|(id, _)| *id)
    }

    pub fn set_original(&mut self, id: PhotoId, image: &Rgba) {
        let buffer = SharedPixelBuffer::<Rgba8Pixel>::clone_from_slice(
            image.as_raw(),
            image.width(),
            image.height(),
        );
        self.original = Some((id, Image::from_rgba8(buffer)));
    }

    pub fn clear_original(&mut self) {
        self.original = None;
    }

    fn screen(&mut self, id: PhotoId) -> Option<Image> {
        let arc = self.screens.get(id)?;
        if let Some((known, img)) = self.converted.get(&id)
            && Arc::ptr_eq(known, &arc)
        {
            return Some(img.clone());
        }
        let (w, h) = (arc.width(), arc.height());
        let buffer = SharedPixelBuffer::<Rgba8Pixel>::clone_from_slice(arc.as_raw(), w, h);
        let img = Image::from_rgba8(buffer);
        self.converted.insert(id, (arc, img.clone()));
        Some(img)
    }
}

/// Fotos dos quadros da lâmina `index`.
fn photos_of(project: &Project, index: usize) -> Vec<PhotoId> {
    project.spreads.get(index).map_or_else(Vec::new, |s| {
        s.frames
            .iter()
            .filter_map(|f| match &f.content {
                FrameContent::Photo { photo, .. } => Some(*photo),
                _ => None,
            })
            .collect()
    })
}
