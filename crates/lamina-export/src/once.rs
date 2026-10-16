//! Foto repetida lida uma vez (spec EI, regra 8; auditoria do Marco 1, M5): na exportação, a
//! mesma foto em vários quadros de uma lâmina sai do disco uma vez só.

use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};

use image::RgbaImage;
use lamina_core::{Photo, PhotoId};
use lamina_render::PhotoSource;

type Loaded = Result<Arc<RgbaImage>, String>;

/// Guarda cada foto já lida durante uma lâmina. Só serve para a impressão, onde a fonte sempre
/// devolve o arquivo inteiro (o tamanho pedido não muda a resposta).
pub struct ReadOnce<'a> {
    inner: &'a dyn PhotoSource,
    seen: Mutex<HashMap<PhotoId, Arc<OnceLock<Loaded>>>>,
}

impl<'a> ReadOnce<'a> {
    pub fn new(inner: &'a dyn PhotoSource) -> Self {
        ReadOnce {
            inner,
            seen: Mutex::new(HashMap::new()),
        }
    }
}

impl PhotoSource for ReadOnce<'_> {
    fn load(&self, photo: &Photo, needed: (u32, u32)) -> Loaded {
        // Os quadros são lidos em paralelo: quem chega depois espera a leitura de quem chegou
        // antes, em vez de ler de novo.
        let cell = match self.seen.lock() {
            Ok(mut seen) => seen.entry(photo.id).or_default().clone(),
            Err(_) => return self.inner.load(photo, needed),
        };
        cell.get_or_init(|| self.inner.load(photo, needed)).clone()
    }
}
