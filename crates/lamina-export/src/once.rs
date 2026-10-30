//! Foto repetida lida uma vez (spec EI, regra 8; auditoria do Marco 1, M5): na exportação, a
//! mesma foto em vários quadros de uma lâmina sai do disco uma vez só.

use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};

use image::RgbaImage;
use lamina_core::{Frame, FrameContent, Photo, PhotoId};
use lamina_render::PhotoSource;

type Loaded = Result<Arc<RgbaImage>, String>;

/// A leitura de uma foto e quantas vezes ela ainda vai ser pedida.
struct Read {
    cell: Arc<OnceLock<Loaded>>,
    left: usize,
}

/// Guarda cada foto já lida durante uma lâmina, até o último quadro que a usa pedir (spec LV,
/// regra 13). Só serve para a impressão, onde a fonte sempre devolve o arquivo inteiro (o tamanho
/// pedido não muda a resposta).
pub struct ReadOnce<'a> {
    inner: &'a dyn PhotoSource,
    uses: HashMap<PhotoId, usize>,
    seen: Mutex<HashMap<PhotoId, Read>>,
}

impl<'a> ReadOnce<'a> {
    /// `frames`, os quadros da lâmina desenhada uma vez.
    pub fn new(inner: &'a dyn PhotoSource, frames: &[Frame]) -> Self {
        let mut uses: HashMap<PhotoId, usize> = HashMap::new();
        for frame in frames {
            if let FrameContent::Photo { photo, .. } = &frame.content {
                *uses.entry(*photo).or_default() += 1;
            }
        }
        ReadOnce {
            inner,
            uses,
            seen: Mutex::new(HashMap::new()),
        }
    }

    /// A célula da foto, que sai da lista no último uso: daí em diante, a foto só fica na memória
    /// enquanto quem pediu estiver com ela. Com a lista quebrada, `None`.
    fn cell(&self, id: PhotoId) -> Option<Arc<OnceLock<Loaded>>> {
        let mut seen = self.seen.lock().ok()?;
        let left = self.uses.get(&id).copied().unwrap_or(usize::MAX);
        let read = seen.entry(id).or_insert_with(|| Read {
            cell: Arc::default(),
            left,
        });
        read.left = read.left.saturating_sub(1);
        let cell = read.cell.clone();
        if read.left == 0 {
            seen.remove(&id);
        }
        Some(cell)
    }
}

impl PhotoSource for ReadOnce<'_> {
    fn load(&self, photo: &Photo, needed: (u32, u32)) -> Loaded {
        // Os quadros são lidos em paralelo: quem chega depois espera a leitura de quem chegou
        // antes, em vez de ler de novo.
        let Some(cell) = self.cell(photo.id) else {
            return self.inner.load(photo, needed);
        };
        cell.get_or_init(|| self.inner.load(photo, needed)).clone()
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};

    use lamina_core::{AlbumSpec, Crop, Dpi, Mm, Project, RelRect};

    use super::*;

    #[test]
    fn lv_b18_a_foto_sai_da_memoria_no_ultimo_uso() {
        let spec =
            AlbumSpec::new(Mm::from_cm(20.0), Mm::from_cm(20.0), Dpi::new(100).unwrap()).unwrap();
        let mut project = Project::new("t", spec);
        let id = project.add_photo("a.jpg", 10, 10);
        let spread = project.add_spread();
        let rect = RelRect::new(0.1, 0.1, 0.2, 0.2).unwrap();
        let content = FrameContent::Photo {
            photo: id,
            crop: Crop::default(),
        };
        project.add_frame(spread, rect, content.clone()).unwrap();
        project.add_frame(spread, rect, content).unwrap();
        let reads = AtomicUsize::new(0);
        let source = |_: &Photo, _: (u32, u32)| {
            reads.fetch_add(1, Ordering::Relaxed);
            Ok(Arc::new(RgbaImage::new(1, 1)))
        };
        let once = ReadOnce::new(&source, &project.spreads[0].frames);
        let photo = project.photo(id).unwrap();
        let first = once.load(photo, (1, 1)).unwrap();
        let second = once.load(photo, (1, 1)).unwrap();
        assert!(Arc::ptr_eq(&first, &second));
        assert_eq!(reads.load(Ordering::Relaxed), 1);
        // Só quem pediu segura a imagem: a lista já soltou.
        drop(first);
        assert_eq!(Arc::strong_count(&second), 1);
    }
}
