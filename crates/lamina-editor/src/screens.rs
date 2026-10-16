//! Imagens de tela na memória (spec JP, regra 6): só as das fotos da lâmina atual e das
//! vizinhas; sem a imagem de tela no cache, a miniatura no lugar até ela ficar pronta.

use std::collections::{BTreeSet, HashMap};
use std::sync::Arc;

use image::RgbaImage;
use lamina_core::{FrameContent, PhotoId, Project};
use rayon::prelude::*;

use crate::Cache;

/// Pixels RGBA de uma imagem de tela, como a janela recebe.
pub type Rgba = RgbaImage;

#[derive(Debug, Default)]
pub struct ScreenImages {
    loaded: HashMap<PhotoId, Loaded>,
}

#[derive(Debug, Clone)]
struct Loaded {
    image: Arc<RgbaImage>,
    /// Veio da miniatura: carregar de novo quando a imagem de tela existir.
    from_thumbnail: bool,
}

impl ScreenImages {
    pub fn new() -> Self {
        Self::default()
    }

    /// Deixa na memória as fotos das lâminas `center - 1` a `center + 1`, lendo em paralelo as
    /// que faltam, e solta as outras.
    pub fn keep_around(&mut self, project: &Project, center: usize, cache: &Cache) {
        let wanted = photos_around(project, center);
        self.loaded
            .retain(|id, l| wanted.contains(id) && !l.from_thumbnail);
        let missing: Vec<PhotoId> = wanted
            .into_iter()
            .filter(|id| !self.loaded.contains_key(id))
            .collect();
        let read: Vec<(PhotoId, Loaded)> = missing
            .into_par_iter()
            .filter_map(|id| Some((id, load(project, id, cache)?)))
            .collect();
        self.loaded.extend(read);
    }

    /// A imagem da foto, se estiver na memória.
    pub fn get(&self, id: PhotoId) -> Option<Arc<RgbaImage>> {
        self.loaded.get(&id).map(|l| Arc::clone(&l.image))
    }

    /// Quantas fotos estão na memória.
    pub fn len(&self) -> usize {
        self.loaded.len()
    }

    pub fn is_empty(&self) -> bool {
        self.loaded.is_empty()
    }
}

/// Fotos dos quadros das lâminas vizinhas de `center` (inclusive), sem repetir.
fn photos_around(project: &Project, center: usize) -> BTreeSet<PhotoId> {
    let range = center.saturating_sub(1)..=center + 1;
    project
        .spreads
        .iter()
        .enumerate()
        .filter(|(i, _)| range.contains(i))
        .flat_map(|(_, s)| &s.frames)
        .filter_map(|f| match &f.content {
            FrameContent::Photo { photo, .. } => Some(*photo),
            _ => None,
        })
        .collect()
}

fn load(project: &Project, id: PhotoId, cache: &Cache) -> Option<Loaded> {
    let photo = project.photo(id).ok()?;
    let (path, from_thumbnail) = match cache.screen(photo) {
        Some(screen) => (screen, false),
        None => (cache.thumbnail(photo)?, true),
    };
    let image = image::open(path).ok()?.into_rgba8();
    Some(Loaded {
        image: Arc::new(image),
        from_thumbnail,
    })
}

/// Gera, em paralelo, a miniatura e a imagem de tela das fotos que ainda não têm, começando
/// pelas da lâmina `first`. Devolve quantas falharam (foto ausente ou ilegível). Roda fora da
/// linha da janela.
pub fn prepare_images(project: &Project, first: usize, cache: &Cache) -> usize {
    let mut order: Vec<PhotoId> = photos_around(project, first).into_iter().collect();
    let rest = project.photos.iter().map(|p| p.id);
    let rest: Vec<PhotoId> = rest.filter(|id| !order.contains(id)).collect();
    order.extend(rest);
    order
        .par_iter()
        .filter_map(|id| project.photo(*id).ok())
        .filter(|p| cache.screen(p).is_none() || cache.thumbnail(p).is_none())
        .filter(|p| cache.ensure_images(p).is_err())
        .count()
}

/// A foto original, na orientação certa, para o quadro em modo recorte (spec RC, regra 9).
/// Roda fora da linha da janela; `None` se a foto não abrir.
pub fn load_original(project: &Project, id: PhotoId) -> Option<Arc<RgbaImage>> {
    let photo = project.photo(id).ok()?;
    lamina_media::load_oriented(&photo.path).ok().map(Arc::new)
}
