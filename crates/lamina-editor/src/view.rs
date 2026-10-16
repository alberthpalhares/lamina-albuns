//! A visão da lâmina (spec JP, regra 5): a lâmina em pixels da tela, centralizada na maior escala
//! que cabe, com cada quadro onde vai sair impresso e a mesma parte da foto que a renderização
//! usa (as funções `bleed` e `region` do `lamina-render`).

use lamina_core::{FrameContent, PageSide, PhotoId, Project, Rect, Rgb};
use lamina_render::{bleed, region};

/// Retângulo em pixels da tela (com fração: a janela desenha sem arredondar).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct PxRect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

/// O que um quadro mostra na tela.
#[derive(Debug, Clone, PartialEq)]
pub enum FrameKind {
    Empty,
    Photo {
        id: PhotoId,
        /// Parte da foto mostrada, em frações da foto inteira (0 a 1): x, y, largura, altura.
        source: PxRect,
        /// A foto não está no caminho salvo (regra 11).
        missing: bool,
    },
    Text,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FrameView {
    /// Quadro já estendido até a sangria quando encosta no corte, como na impressão.
    pub rect: PxRect,
    pub kind: FrameKind,
}

/// A lâmina pronta para desenhar.
#[derive(Debug, Clone, PartialEq)]
pub struct SpreadView {
    /// Pixels da tela por milímetro.
    pub scale: f64,
    /// A lâmina inteira com a sangria (o arquivo exportado).
    pub canvas: PxRect,
    /// O corte final.
    pub trim: PxRect,
    /// Zona segura da página esquerda e da direita.
    pub safe: [PxRect; 2],
    /// Posição x da lombada.
    pub spine_x: f64,
    /// Páginas sem conteúdo (lâmina de ponta com página simples).
    pub inactive: Vec<PxRect>,
    pub background: Rgb,
    pub frames: Vec<FrameView>,
}

/// Área da tela onde a lâmina é desenhada, com uma margem livre em volta.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Area {
    pub width: f64,
    pub height: f64,
    pub margin: f64,
}

/// A lâmina `index` (a partir de 0) desenhada em `area`; `None` se a lâmina não existe ou a área
/// é pequena demais.
pub fn spread_view(
    project: &Project,
    index: usize,
    area: Area,
    missing: &[PhotoId],
) -> Option<SpreadView> {
    let spread = project.spreads.get(index)?;
    let spec = &project.spec;
    let canvas = spec.canvas_rect();
    let free = (
        area.width - 2.0 * area.margin,
        area.height - 2.0 * area.margin,
    );
    if free.0 <= 0.0 || free.1 <= 0.0 {
        return None;
    }
    let scale = (free.0 / canvas.width.get()).min(free.1 / canvas.height.get());
    let origin = (
        (area.width - canvas.width.get() * scale) / 2.0,
        (area.height - canvas.height.get() * scale) / 2.0,
    );
    let px = |r: Rect| PxRect {
        x: origin.0 + r.x.get() * scale,
        y: origin.1 + r.y.get() * scale,
        width: r.width.get() * scale,
        height: r.height.get() * scale,
    };
    let trim = spec.trim_rect();
    let active = spec.edges().active_pages(index, project.spreads.len());
    let inactive = [
        (active.left, PageSide::Left),
        (active.right, PageSide::Right),
    ]
    .into_iter()
    .filter(|(on, _)| !on)
    .map(|(_, side)| px(spec.page_canvas_rect(side)))
    .collect();
    let frames = spread
        .frames
        .iter()
        .map(|frame| {
            let rect = project.frame_rect(frame);
            let bled = bleed(spec, rect);
            let kind = match &frame.content {
                FrameContent::Empty => FrameKind::Empty,
                FrameContent::Text { .. } => FrameKind::Text,
                FrameContent::Photo { photo, crop } => {
                    let size = project
                        .photo(*photo)
                        .map(|p| (f64::from(p.width_px), f64::from(p.height_px)))
                        .unwrap_or((1.0, 1.0));
                    let r = region(crop, rect, &bled, size);
                    FrameKind::Photo {
                        id: *photo,
                        source: PxRect {
                            x: r.x / size.0,
                            y: r.y / size.1,
                            width: r.width / size.0,
                            height: r.height / size.1,
                        },
                        missing: missing.contains(photo),
                    }
                }
            };
            FrameView {
                rect: px(bled.rect),
                kind,
            }
        })
        .collect();
    Some(SpreadView {
        scale,
        canvas: px(canvas),
        trim: px(trim),
        safe: [
            px(spec.safe_rect(PageSide::Left)),
            px(spec.safe_rect(PageSide::Right)),
        ],
        spine_x: origin.0 + (trim.x.get() + trim.width.get() / 2.0) * scale,
        inactive,
        background: spread.background,
        frames,
    })
}
