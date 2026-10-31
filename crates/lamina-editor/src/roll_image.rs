//! A imagem da lâmina para o rolo (spec RL, regras 3 a 5 e 10; ADR 0012, decisões 2, 3 e 6): a
//! prévia do renderizador único na largura pedida, mais as marcas de tela, com as fotos lidas só
//! do cache (a miniatura quando basta, a imagem de tela quando não; nunca o original).

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use image::{Rgba, RgbaImage};
use lamina_core::byte_cache::ByteCache;
use lamina_core::{FrameContent, Photo, PhotoId, Project};
use lamina_media::THUMB_SIZE;
use lamina_render::{
    EMPTY_GRAY, PhotoSource, RenderError, RenderTarget, ScreenMarks, draw_screen_marks,
    render_spread_with,
};

use crate::Cache;
use crate::motion::{RollAsk, Tier};

/// Lado da imagem cinza que fica no quadro de uma foto ainda sem miniatura no cache.
const GRAY_SIDE: u32 = 8;
/// Espessura das marcas na lâmina grande, em pixels lógicos (spec JE, seção 4).
const MARK_LINE: f32 = 1.5;

/// As fotos de uma lâmina, só do cache (regra 4). Marca a imagem como incompleta quando faltou a
/// imagem de tela (ou a miniatura) que um quadro pedia.
#[derive(Debug)]
pub struct RollSource {
    tier: Tier,
    thumbs: BTreeMap<PhotoId, PathBuf>,
    screens: BTreeMap<PhotoId, PathBuf>,
    incomplete: AtomicBool,
    images: Option<Arc<RollImages>>,
}

/// Quantos bytes de imagens já lidas o rolo guarda (spec LV, regra 10).
pub const ROLL_IMAGES_BYTES: usize = 128 * 1024 * 1024;

/// As imagens do cache já lidas do disco, com limite em bytes. A chave é o caminho, que já leva a
/// impressão digital da foto: uma imagem guardada nunca fica velha.
#[derive(Debug)]
pub struct RollImages(Mutex<ByteCache<PathBuf, Arc<RgbaImage>>>);

impl RollImages {
    pub fn new(limit: usize) -> Self {
        RollImages(Mutex::new(ByteCache::new(limit)))
    }

    fn locked(&self) -> MutexGuard<'_, ByteCache<PathBuf, Arc<RgbaImage>>> {
        self.0.lock().unwrap_or_else(PoisonError::into_inner)
    }

    fn get(&self, path: &Path) -> Option<Arc<RgbaImage>> {
        self.locked().get(&path.to_path_buf())
    }

    fn insert(&self, path: PathBuf, image: Arc<RgbaImage>) {
        let bytes = image.as_raw().len();
        self.locked().insert(path, image, bytes);
    }

    /// Outro álbum: as imagens do anterior vão embora.
    pub fn clear(&self) {
        self.locked().clear();
    }

    /// Quantos bytes estão guardados agora.
    pub fn bytes(&self) -> usize {
        self.locked().bytes()
    }
}

impl RollSource {
    /// A fonte das fotos da lâmina `index`: procura no cache só as fotos dela.
    pub fn new(project: &Project, index: usize, cache: &Cache, tier: Tier) -> Self {
        let photos = project.spreads.get(index).map_or(&[][..], |s| &s.frames);
        let photos: Vec<&Photo> = photos
            .iter()
            .filter_map(|f| match &f.content {
                FrameContent::Photo { photo, .. } => project.photo(*photo).ok(),
                _ => None,
            })
            .collect();
        let found = |get: &dyn Fn(&Photo) -> Option<PathBuf>| {
            photos
                .iter()
                .filter_map(|p| Some((p.id, get(p)?)))
                .collect::<BTreeMap<_, _>>()
        };
        let screens = match tier {
            Tier::Small => BTreeMap::new(),
            Tier::Screen => found(&|p| cache.screen(p)),
        };
        RollSource {
            tier,
            thumbs: found(&|p| cache.thumbnail(p)),
            screens,
            incomplete: AtomicBool::new(false),
            images: None,
        }
    }

    /// Alguma foto saiu com menos do que pedia.
    pub fn incomplete(&self) -> bool {
        self.incomplete.load(Ordering::Relaxed)
    }

    /// Passa a guardar as imagens lidas em `images`: pintar a mesma lâmina de novo não lê o disco.
    pub fn sharing(mut self, images: Arc<RollImages>) -> Self {
        self.images = Some(images);
        self
    }

    fn open(&self, path: Option<&PathBuf>) -> Option<Arc<RgbaImage>> {
        let path = path?;
        if let Some(image) = self.images.as_ref().and_then(|i| i.get(path)) {
            return Some(image);
        }
        let image = Arc::new(lamina_media::to_rgba(image::open(path).ok()?));
        if let Some(images) = &self.images {
            images.insert(path.clone(), image.clone());
        }
        Some(image)
    }
}

impl PhotoSource for RollSource {
    fn load(&self, photo: &Photo, needed: (u32, u32)) -> Result<Arc<RgbaImage>, String> {
        let small = needed.0.max(needed.1) <= THUMB_SIZE;
        let thumb = || self.open(self.thumbs.get(&photo.id));
        let best = if small || self.tier == Tier::Small {
            thumb()
        } else {
            self.open(self.screens.get(&photo.id)).or_else(|| {
                self.incomplete.store(true, Ordering::Relaxed);
                thumb()
            })
        };
        Ok(best.unwrap_or_else(|| {
            self.incomplete.store(true, Ordering::Relaxed);
            let gray = Rgba([EMPTY_GRAY[0], EMPTY_GRAY[1], EMPTY_GRAY[2], 255]);
            Arc::new(RgbaImage::from_pixel(GRAY_SIDE, GRAY_SIDE, gray))
        }))
    }
}

/// Uma imagem de lâmina pronta.
#[derive(Debug, Clone)]
pub struct SpreadImage {
    pub pixels: RgbaImage,
    /// Feita com todas as imagens de tela que pedia (regra 4).
    pub complete: bool,
}

/// O tamanho em que a imagem vai aparecer: a largura dela em pixels e a largura do lugar em
/// pixels lógicos (para as marcas terem a espessura da lâmina grande).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ImageSize {
    pub width: u32,
    pub shown: f64,
}

/// A imagem da lâmina `index` com as marcas de tela (regra 3): a prévia de `render_spread_with`
/// na largura pedida, com a fonte `source`; `missing` são as fotos que não estão no caminho salvo.
pub fn render_spread_image(
    project: &Project,
    (index, size): (usize, ImageSize),
    source: &RollSource,
    missing: &[PhotoId],
) -> Result<SpreadImage, RenderError> {
    let target = RenderTarget::Preview {
        max_side: preview_side(project, size.width),
    };
    let preview = render_spread_with(project, index, target, source, lamina_render::Fill::Pages)?;
    #[allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]
    let line = (f64::from(MARK_LINE) * f64::from(preview.width()) / size.shown.max(1.0)) as f32;
    let marks = ScreenMarks {
        missing,
        line: line.max(0.75),
    };
    let pixels = draw_screen_marks(preview, project, index, &marks)?;
    Ok(SpreadImage {
        pixels,
        complete: !source.incomplete(),
    })
}

/// O lado maior da prévia para ela sair com `width` pixels de largura.
fn preview_side(project: &Project, width: u32) -> u32 {
    let canvas = project.spec.canvas_rect();
    let (w, h) = (canvas.width.get(), canvas.height.get());
    let side = f64::from(width) * w.max(h) / w.max(1e-9);
    // Arredondado e preso a um u32 com folga.
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let side = side.round().clamp(1.0, f64::from(u32::MAX / 2)) as u32;
    side
}

/// Quem desenha as imagens do rolo fora da linha da janela, com as próprias linhas (regra 10: no
/// G0, todas as linhas da máquina faziam a janela perder quadros).
#[derive(Debug)]
pub struct RollPainter {
    pool: Option<rayon::ThreadPool>,
    cache: Cache,
    images: Arc<RollImages>,
}

impl RollPainter {
    /// Um desenhista com `threads` linhas, lendo do `cache`.
    pub fn new(threads: usize, cache: Cache) -> Self {
        let pool = rayon::ThreadPoolBuilder::new()
            .num_threads(threads.max(1))
            .thread_name(|i| format!("lamina-rolo-{i}"))
            .build()
            .ok();
        let images = Arc::new(RollImages::new(ROLL_IMAGES_BYTES));
        RollPainter {
            pool,
            cache,
            images,
        }
    }

    /// As imagens guardadas, para a janela esvaziar ao trocar de álbum.
    pub fn images(&self) -> Arc<RollImages> {
        self.images.clone()
    }

    /// Roda `f` nas linhas do desenhista (o que ele paraleliza fica nelas).
    pub fn run<R: Send>(&self, f: impl FnOnce() -> R + Send) -> R {
        match &self.pool {
            Some(pool) => pool.install(f),
            None => f(),
        }
    }

    /// A imagem do pedido `ask`; `shown` é a largura do lugar em pixels lógicos.
    pub fn paint(
        &self,
        project: &Project,
        (ask, shown): (&RollAsk, f64),
        missing: &[PhotoId],
    ) -> Result<SpreadImage, RenderError> {
        self.run(|| {
            let source = RollSource::new(project, ask.index, &self.cache, ask.tier)
                .sharing(self.images.clone());
            let size = ImageSize {
                width: ask.width,
                shown,
            };
            render_spread_image(project, (ask.index, size), &source, missing)
        })
    }
}
