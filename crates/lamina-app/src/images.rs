//! As imagens já convertidas para o Slint: miniaturas (faixa e navegador) e imagens de tela
//! (lâmina grande). Na memória ficam só as imagens de tela da lâmina atual e das vizinhas (spec
//! JP, regra 6), lidas por um trabalhador; a lâmina aparece com as miniaturas e cada foto nítida
//! entra quando chega (spec MV, regra 1). Quem decide o que pedir é o `ScreenPlan` do editor.

use std::collections::HashMap;

use lamina_core::{FrameContent, PhotoId, Project};
use lamina_editor::{Arrival, Cache, ScreenPlan};
use slint::{Image, Rgba8Pixel, SharedPixelBuffer};

use crate::loader::Loader;
use crate::mailbox::Mailbox;
use crate::state::App;

pub type Pixels = SharedPixelBuffer<Rgba8Pixel>;

pub struct Images {
    /// Sobe a cada álbum novo: fotos de um álbum anterior que chegarem são ignoradas.
    epoch: u64,
    plan: ScreenPlan,
    loader: Loader,
    screens: HashMap<PhotoId, Image>,
    /// Chegaram durante um deslize e entram quando ele termina (MV-B26).
    held: HashMap<PhotoId, Image>,
    thumbs: HashMap<PhotoId, Image>,
    /// A foto original do quadro em modo recorte (spec RC, regra 9).
    original: Option<(PhotoId, Image)>,
    /// Chegou foto nítida nova desde o último desenho da lâmina.
    pub(crate) arrived: bool,
}

impl Images {
    pub fn new(cache: &Cache, mailbox: &Mailbox) -> Self {
        Images {
            epoch: 0,
            plan: ScreenPlan::default(),
            loader: Loader::new(cache, mailbox),
            screens: HashMap::new(),
            held: HashMap::new(),
            thumbs: HashMap::new(),
            original: None,
            arrived: false,
        }
    }

    /// Esquece tudo (álbum novo).
    pub fn clear(&mut self) {
        self.epoch += 1;
        self.plan = ScreenPlan::default();
        self.screens.clear();
        self.held.clear();
        self.thumbs.clear();
        self.original = None;
    }

    /// O cache ganhou imagens: as fotos que não tinham imagem de tela podem ser pedidas de novo.
    pub fn recheck(&mut self) {
        self.plan.recheck();
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

    /// As imagens das fotos da lâmina `index`: a de tela, ou a miniatura enquanto ela não chega.
    /// Pede ao trabalhador as que faltam (a lâmina atual primeiro) e solta as que saíram.
    pub fn spread(
        &mut self,
        project: &Project,
        index: usize,
        cache: &Cache,
    ) -> HashMap<PhotoId, Image> {
        let asked = self.plan.go(project, index);
        for id in &asked.release {
            self.screens.remove(id);
            self.held.remove(id);
        }
        if !asked.photos.is_empty() {
            let items = asked
                .photos
                .iter()
                .filter_map(|id| Some((*id, project.photo(*id).ok()?.clone())))
                .collect();
            self.loader.ask(self.epoch, index, items);
        }
        let mut images = HashMap::new();
        for id in photos_of(project, index) {
            let original = self
                .original
                .as_ref()
                .filter(|(o, _)| *o == id)
                .map(|(_, img)| img.clone());
            let img = original
                .or_else(|| self.screens.get(&id).cloned())
                .or_else(|| self.thumb(project, id, cache));
            if let Some(img) = img {
                images.insert(id, img);
            }
        }
        images
    }

    /// Uma foto de tela chegou do trabalhador (`None`: não há imagem de tela no cache).
    pub fn screen_arrived(&mut self, epoch: u64, id: PhotoId, pixels: Option<Pixels>) {
        if epoch != self.epoch {
            return;
        }
        let Some(pixels) = pixels else {
            self.plan.failed(id);
            return;
        };
        // Ainda não há deslize de navegação: a foto entra na hora (MV-B26 liga o deslize).
        match self.plan.arrived(id, false) {
            Arrival::Show => {
                self.screens.insert(id, Image::from_rgba8(pixels));
                self.arrived = true;
            }
            Arrival::Hold => {
                self.held.insert(id, Image::from_rgba8(pixels));
            }
            Arrival::Discard => {}
        }
    }

    pub fn original_id(&self) -> Option<PhotoId> {
        self.original.as_ref().map(|(id, _)| *id)
    }

    /// A foto original do recorte, já convertida pelo trabalhador.
    pub fn set_original(&mut self, id: PhotoId, pixels: Pixels) {
        self.original = Some((id, Image::from_rgba8(pixels)));
    }

    pub fn clear_original(&mut self) {
        self.original = None;
    }
}

/// Depois de tratar as mensagens: se chegou foto nítida, a lâmina é redesenhada uma vez só.
pub fn after_arrivals(app: &mut App) {
    if std::mem::take(&mut app.images.arrived) {
        crate::crop::refresh_spread(app);
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
