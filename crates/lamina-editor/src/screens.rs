//! Imagens de tela (spec JP, regra 6; spec MV, regra 1): só as das fotos da lâmina atual e das
//! vizinhas, lidas fora da linha da janela. O [`ScreenPlan`] decide o que pedir, em que ordem, o
//! que aceitar quando chega e o que soltar; a janela mostra a miniatura até a foto nítida chegar.

use std::collections::BTreeSet;
use std::sync::Arc;

use image::RgbaImage;
use lamina_core::{FrameContent, Photo, PhotoId, Project};
use rayon::prelude::*;

use crate::Cache;

/// Pixels RGBA de uma imagem de tela, como a janela recebe.
pub type Rgba = RgbaImage;

/// O que pedir ao ir para uma lâmina.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Requests {
    /// As fotos a ler, na ordem: as da lâmina atual primeiro, depois as das vizinhas.
    pub photos: Vec<PhotoId>,
    /// Quantas do começo de `photos` são da lâmina atual.
    pub now: usize,
    /// As fotos que deixaram de ser vizinhas: a janela solta as imagens delas.
    pub release: Vec<PhotoId>,
}

/// O que fazer com uma foto nítida que acabou de chegar.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Arrival {
    /// Entra na tela agora.
    Show,
    /// Chegou no meio de um deslize: entra quando ele terminar (regra 1, MV-B26).
    Hold,
    /// Não é mais da lâmina atual nem das vizinhas (MV-B19).
    Discard,
}

/// O plano das imagens de tela: o que está na janela, o que foi pedido e o que espera o deslize.
#[derive(Debug, Default)]
pub struct ScreenPlan {
    center: Option<usize>,
    wanted: BTreeSet<PhotoId>,
    loaded: BTreeSet<PhotoId>,
    asked: BTreeSet<PhotoId>,
    held: Vec<PhotoId>,
    /// Sem imagem de tela no cache: não pedir de novo até o cache mudar.
    absent: BTreeSet<PhotoId>,
}

impl ScreenPlan {
    /// Vai para a lâmina `center`: as fotos dela e das vizinhas que ainda faltam, e as que saem.
    /// Mudou de lâmina, os pedidos que não chegaram são feitos de novo, na ordem nova (quem lê
    /// troca a fila inteira).
    pub fn go(&mut self, project: &Project, center: usize) -> Requests {
        if self.center != Some(center) {
            self.center = Some(center);
            self.asked.clear();
        }
        let order = photos_around(project, center);
        let now = photos_of(project, center).len();
        self.wanted = order.iter().copied().collect();
        let release: Vec<PhotoId> = self
            .loaded
            .iter()
            .chain(&self.held)
            .filter(|id| !self.wanted.contains(id))
            .copied()
            .collect();
        let wanted = &self.wanted;
        self.loaded.retain(|id| wanted.contains(id));
        self.held.retain(|id| wanted.contains(id));
        self.asked.retain(|id| wanted.contains(id));
        let mut photos = Vec::new();
        let mut first = 0;
        for (i, id) in order.into_iter().enumerate() {
            let known = self.loaded.contains(&id)
                || self.held.contains(&id)
                || self.asked.contains(&id)
                || self.absent.contains(&id);
            if !known {
                self.asked.insert(id);
                photos.push(id);
                if i < now {
                    first += 1;
                }
            }
        }
        Requests {
            photos,
            now: first,
            release,
        }
    }

    /// A foto nítida `id` chegou; `sliding` diz se a lâmina está no meio de um deslize.
    pub fn arrived(&mut self, id: PhotoId, sliding: bool) -> Arrival {
        self.asked.remove(&id);
        if !self.wanted.contains(&id) {
            return Arrival::Discard;
        }
        if sliding {
            if !self.held.contains(&id) {
                self.held.push(id);
            }
            Arrival::Hold
        } else {
            self.loaded.insert(id);
            Arrival::Show
        }
    }

    /// O deslize terminou: as fotos guardadas entram agora.
    pub fn slide_ended(&mut self) -> Vec<PhotoId> {
        let held = std::mem::take(&mut self.held);
        self.loaded.extend(held.iter().copied());
        held
    }

    /// A foto não tem imagem de tela no cache (ou não abriu): fica a miniatura.
    pub fn failed(&mut self, id: PhotoId) {
        self.asked.remove(&id);
        self.absent.insert(id);
    }

    /// O cache mudou (imagens recém-geradas): as que faltavam podem ser pedidas de novo.
    pub fn recheck(&mut self) {
        self.absent.clear();
    }

    /// A foto nítida já está na tela.
    pub fn is_sharp(&self, id: PhotoId) -> bool {
        self.loaded.contains(&id)
    }
}

/// Fotos dos quadros da lâmina `index`, na ordem dos quadros e sem repetir.
fn photos_of(project: &Project, index: usize) -> Vec<PhotoId> {
    let mut out = Vec::new();
    for frame in project.spreads.get(index).map_or(&[][..], |s| &s.frames) {
        if let FrameContent::Photo { photo, .. } = &frame.content
            && !out.contains(photo)
        {
            out.push(*photo);
        }
    }
    out
}

/// Fotos da lâmina `center` e depois das vizinhas (a anterior e a seguinte), sem repetir.
fn photos_around(project: &Project, center: usize) -> Vec<PhotoId> {
    let mut out = photos_of(project, center);
    let before = center.checked_sub(1);
    for index in before.into_iter().chain(Some(center + 1)) {
        for id in photos_of(project, index) {
            if !out.contains(&id) {
                out.push(id);
            }
        }
    }
    out
}

/// A imagem de tela da foto, lida do cache; `None` se ela ainda não existe ou não abre. Roda
/// fora da linha da janela.
pub fn load_screen(photo: &Photo, cache: &Cache) -> Option<Arc<RgbaImage>> {
    let path = cache.screen(photo)?;
    image::open(path).ok().map(|i| Arc::new(i.into_rgba8()))
}

/// Gera, em paralelo, a miniatura e a imagem de tela das fotos que ainda não têm, começando
/// pelas da lâmina `first`. Devolve quantas falharam (foto ausente ou ilegível). Roda fora da
/// linha da janela.
pub fn prepare_images(project: &Project, first: usize, cache: &Cache) -> usize {
    let mut order = photos_around(project, first);
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
