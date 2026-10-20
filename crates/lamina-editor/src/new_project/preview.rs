//! O que as etapas desenham (spec NP, seção 4 e regra 7): a lâmina em escala da etapa 1 e as
//! três lâminas de exemplo da etapa 2, montadas pelo nosso gerador com fotos fictícias.

use std::sync::Arc;

use image::{Rgba, RgbaImage};
use lamina_core::{AlbumEdge, AlbumSpec, DesignMode, Photo, Project};
use lamina_layout::{BuildOptions, auto_build};
use lamina_render::RenderTarget;

use super::{AlbumStyle, apply_look, put_style};

/// Lado maior de cada lâmina da prévia, em pixels.
pub const PREVIEW_SIDE: u32 = 480;
/// Cinza neutro das fotos fictícias.
const NEUTRAL: Rgba<u8> = Rgba([150, 150, 150, 255]);

/// A lâmina aberta em frações da largura e da altura do canvas (corte + sangria), para o desenho
/// em escala: onde fica o corte, a zona segura de cada página e a dobra.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Drawing {
    /// Largura ÷ altura do canvas.
    pub aspect: f64,
    /// O corte: x, y, largura e altura.
    pub trim: [f64; 4],
    /// A zona segura da página esquerda e da direita.
    pub safe_left: [f64; 4],
    pub safe_right: [f64; 4],
    /// Página simples no início (a esquerda fica vazia) e no fim (a direita fica vazia).
    pub starts_single: bool,
    pub ends_single: bool,
}

/// O desenho da especificação.
pub fn drawing(spec: &AlbumSpec) -> Drawing {
    let canvas = spec.canvas_rect();
    let (cw, ch) = (canvas.width.get(), canvas.height.get());
    let frac = |r: lamina_core::Rect| {
        [
            (r.x.get() - canvas.x.get()) / cw,
            (r.y.get() - canvas.y.get()) / ch,
            r.width.get() / cw,
            r.height.get() / ch,
        ]
    };
    Drawing {
        aspect: cw / ch,
        trim: frac(spec.trim_rect()),
        safe_left: frac(spec.safe_rect(lamina_core::PageSide::Left)),
        safe_right: frac(spec.safe_rect(lamina_core::PageSide::Right)),
        starts_single: spec.edges().start == AlbumEdge::SinglePage,
        ends_single: spec.edges().end == AlbumEdge::SinglePage,
    }
}

/// Três lâminas de exemplo (com 2, 3 e 5 fotos fictícias 3:2 e 2:3) montadas com a montagem e a
/// aparência escolhidas, desenhadas pelo renderizador da prévia. Lâmina que não sai fica de fora.
pub fn mount_preview(spec: &AlbumSpec, style: &AlbumStyle, lay_flat: bool) -> Vec<RgbaImage> {
    let mode = if lay_flat {
        DesignMode::Spread
    } else {
        DesignMode::Page
    };
    let spec = spec
        .clone()
        .with_mode(mode)
        .with_edges(AlbumEdge::FullSpread, AlbumEdge::FullSpread);
    let mut sample = Project::new("Prévia", spec);
    for i in 0..10 {
        let (w, h) = if i % 3 == 2 {
            (2000, 3000)
        } else {
            (3000, 2000)
        };
        sample.add_photo(format!("exemplo-{i}.jpg"), w, h);
    }
    put_style(&mut sample, style);
    let Ok(built) = auto_build(&sample, 3, &BuildOptions::default()) else {
        return Vec::new();
    };
    let mut project = built.project;
    apply_look(&mut project);
    let target = RenderTarget::Preview {
        max_side: PREVIEW_SIDE,
    };
    (0..project.spreads.len())
        .filter_map(|i| lamina_render::render_spread(&project, i, target, &neutral).ok())
        .collect()
}

/// Uma foto fictícia cinza, pequena, na proporção da foto.
fn neutral(photo: &Photo, _: (u32, u32)) -> Result<Arc<RgbaImage>, String> {
    let (w, h) = if photo.width_px >= photo.height_px {
        (30, 20)
    } else {
        (20, 30)
    };
    Ok(Arc::new(RgbaImage::from_pixel(w, h, NEUTRAL)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::album::SpecFields;
    use lamina_core::{CropLevel, Mounting};

    #[test]
    fn desenho_em_escala_acompanha_a_sangria_e_a_zona_segura() {
        let spec = SpecFields::default().to_spec().unwrap();
        let d = drawing(&spec);
        // 606 × 306 mm: 3 mm de sangria em volta do corte de 600 × 300 mm.
        assert!((d.aspect - 606.0 / 306.0).abs() < 1e-9);
        assert!((d.trim[0] - 3.0 / 606.0).abs() < 1e-9);
        assert!((d.trim[2] - 600.0 / 606.0).abs() < 1e-9);
        assert!(d.safe_left[0] > d.trim[0] && d.safe_right[0] > 0.5);
        let fields = SpecFields {
            bleed_mm: 10.0,
            starts_single: true,
            ..SpecFields::default()
        };
        let wider = drawing(&fields.to_spec().unwrap());
        assert!(wider.trim[0] > d.trim[0]);
        assert!(wider.starts_single && !wider.ends_single);
    }

    #[test]
    fn previa_da_montagem_tem_tres_laminas_e_muda_com_ela() {
        let spec = SpecFields::default().to_spec().unwrap();
        let free = mount_preview(&spec, &AlbumStyle::default(), true);
        assert_eq!(free.len(), 3);
        assert_eq!(free[0].width(), PREVIEW_SIDE);
        let whole = AlbumStyle {
            mounting: Mounting {
                crop: CropLevel::Whole,
                ..Mounting::default()
            },
            ..AlbumStyle::default()
        };
        let mut gray = AlbumStyle::default();
        gray.look.background = lamina_core::Rgb {
            r: 40,
            g: 40,
            b: 40,
        };
        let darker = mount_preview(&spec, &gray, true);
        assert_ne!(darker[0], free[0], "o fundo aparece na prévia");
        assert_eq!(mount_preview(&spec, &whole, false).len(), 3);
    }
}
