//! O recorte da foto no quadro (spec RC): o zoom e o ponto da foto no centro do quadro, a região
//! visível e os gestos do modo recorte.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::CoreError;

/// Como a foto é enquadrada dentro do quadro.
///
/// Com `zoom = 1` a foto cobre o quadro inteiro (sem faixas vazias), cortando o que sobra.
/// `center_x`/`center_y` dizem qual ponto da foto (em frações 0..=1) fica no centro do quadro.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Crop {
    pub zoom: f64,
    pub center_x: f64,
    pub center_y: f64,
}

impl Default for Crop {
    fn default() -> Self {
        Crop {
            zoom: 1.0,
            center_x: 0.5,
            center_y: 0.5,
        }
    }
}

/// Região retangular em pixels da foto.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PixelRegion {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

impl Crop {
    /// Zoom máximo permitido pela interface.
    pub const MAX_ZOOM: f64 = 8.0;

    /// Pedaço da foto (em pixels da foto) que aparece num quadro de proporção `frame_aspect`
    /// (largura ÷ altura). Com zoom 1 a foto cobre o quadro; o centro é ajustado para que a
    /// região nunca saia da foto.
    pub fn visible_region(&self, photo_w: f64, photo_h: f64, frame_aspect: f64) -> PixelRegion {
        let (full_w, full_h) = if photo_w / photo_h > frame_aspect {
            (photo_h * frame_aspect, photo_h)
        } else {
            (photo_w, photo_w / frame_aspect)
        };
        let zoom = self.zoom.max(1.0);
        let (width, height) = (full_w / zoom, full_h / zoom);
        let x = (self.center_x * photo_w - width / 2.0).clamp(0.0, photo_w - width);
        let y = (self.center_y * photo_h - height / 2.0).clamp(0.0, photo_h - height);
        PixelRegion {
            x,
            y,
            width,
            height,
        }
    }

    /// Novo recorte depois de arrastar a foto `dx`/`dy` frações do quadro (arrastar para a
    /// direita mostra mais do lado esquerdo da foto).
    pub fn panned(&self, photo_w: f64, photo_h: f64, frame_aspect: f64, dx: f64, dy: f64) -> Crop {
        let region = self.visible_region(photo_w, photo_h, frame_aspect);
        let moved = Crop {
            center_x: self.center_x - dx * region.width / photo_w,
            center_y: self.center_y - dy * region.height / photo_h,
            ..*self
        };
        moved.clamped(photo_w, photo_h, frame_aspect)
    }

    /// Novo recorte com o zoom multiplicado por `factor`, entre 1 e [`Crop::MAX_ZOOM`].
    pub fn zoomed(&self, photo_w: f64, photo_h: f64, frame_aspect: f64, factor: f64) -> Crop {
        let zoomed = Crop {
            zoom: (self.zoom * factor).clamp(1.0, Self::MAX_ZOOM),
            ..*self
        };
        zoomed.clamped(photo_w, photo_h, frame_aspect)
    }

    /// Traz o centro para o ponto que a região visível realmente usa.
    fn clamped(&self, photo_w: f64, photo_h: f64, frame_aspect: f64) -> Crop {
        let region = self.visible_region(photo_w, photo_h, frame_aspect);
        Crop {
            center_x: (region.x + region.width / 2.0) / photo_w,
            center_y: (region.y + region.height / 2.0) / photo_h,
            ..*self
        }
    }

    pub fn validate(&self) -> Result<(), CoreError> {
        if !self.zoom.is_finite() || self.zoom < 1.0 {
            return Err(CoreError::InvalidCrop(format!(
                "zoom deve ser >= 1, recebido {}",
                self.zoom
            )));
        }
        let in_unit = |v: f64| v.is_finite() && (0.0..=1.0).contains(&v);
        if !in_unit(self.center_x) || !in_unit(self.center_y) {
            return Err(CoreError::InvalidCrop(format!(
                "centro fora da foto: ({}, {})",
                self.center_x, self.center_y
            )));
        }
        Ok(())
    }

    /// Este recorte como o modo recorte o deixa (spec IA, `recortar`): conferido por
    /// [`Crop::validate`], com o zoom até [`Crop::MAX_ZOOM`], e o centro trazido para o ponto
    /// que a região visível realmente usa.
    pub fn checked(
        &self,
        photo_w: f64,
        photo_h: f64,
        frame_aspect: f64,
    ) -> Result<Crop, CoreError> {
        self.validate()?;
        if self.zoom > Self::MAX_ZOOM {
            return Err(CoreError::InvalidCrop(format!(
                "zoom deve ser <= {}, recebido {}",
                Self::MAX_ZOOM,
                self.zoom
            )));
        }
        Ok(self.clamped(photo_w, photo_h, frame_aspect))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const W: f64 = 6000.0;
    const H: f64 = 4000.0;

    fn region(x: f64, y: f64, width: f64, height: f64) -> PixelRegion {
        PixelRegion {
            x,
            y,
            width,
            height,
        }
    }

    #[test]
    fn foto_horizontal_em_quadro_quadrado_corta_as_laterais() {
        let r = Crop::default().visible_region(W, H, 1.0);
        assert_eq!(r, region(1000.0, 0.0, 4000.0, 4000.0));
    }

    #[test]
    fn foto_horizontal_em_quadro_mais_largo_corta_em_cima_e_embaixo() {
        let r = Crop::default().visible_region(W, H, 3.0);
        assert_eq!(r, region(0.0, 1000.0, 6000.0, 2000.0));
    }

    #[test]
    fn zoom_2_mostra_metade_de_cada_lado() {
        let crop = Crop {
            zoom: 2.0,
            ..Crop::default()
        };
        assert_eq!(
            crop.visible_region(W, H, 1.0),
            region(2000.0, 1000.0, 2000.0, 2000.0)
        );
    }

    #[test]
    fn centro_na_borda_nao_deixa_a_regiao_sair_da_foto() {
        let crop = Crop {
            center_x: 0.0,
            ..Crop::default()
        };
        assert_eq!(crop.visible_region(W, H, 1.0).x, 0.0);
    }

    #[test]
    fn arrastar_para_a_direita_mostra_o_lado_esquerdo() {
        let crop = Crop::default().panned(W, H, 1.0, 0.1, 0.0);
        // A região tem 4000 px de largura; 10% do quadro = 400 px de foto.
        assert_eq!(crop.visible_region(W, H, 1.0).x, 600.0);
    }

    #[test]
    fn arrastar_alem_da_borda_para_no_limite() {
        let crop = Crop::default().panned(W, H, 1.0, 5.0, 0.0);
        assert_eq!(crop.visible_region(W, H, 1.0).x, 0.0);
        assert!((crop.center_x - 4000.0 / 2.0 / W).abs() < 1e-12);
    }

    #[test]
    fn zoom_respeita_os_limites() {
        let crop = Crop::default();
        assert_eq!(crop.zoomed(W, H, 1.0, 0.5).zoom, 1.0);
        assert_eq!(crop.zoomed(W, H, 1.0, 100.0).zoom, Crop::MAX_ZOOM);
        assert_eq!(crop.zoomed(W, H, 1.0, 1.5).zoom, 1.5);
    }

    #[test]
    fn recorte_conferido_fica_como_no_modo_recorte() {
        let wanted = Crop {
            zoom: 1.5,
            center_x: 0.3,
            center_y: 0.5,
        };
        assert_eq!(wanted.checked(W, H, 1.0).unwrap(), wanted);
        // Centro perto demais da borda: vai para onde a região visível encosta nela.
        let edge = Crop {
            center_x: 0.05,
            ..wanted
        };
        let width = 4000.0 / 1.5;
        let fixed = edge.checked(W, H, 1.0).unwrap();
        assert!((fixed.center_x - width / 2.0 / W).abs() < 1e-12);
        assert_eq!(fixed.visible_region(W, H, 1.0).x, 0.0);
    }

    #[test]
    fn recorte_fora_dos_limites_recusa() {
        for (zoom, center_x) in [(0.5, 0.5), (9.0, 0.5), (f64::NAN, 0.5), (1.0, 1.5)] {
            let crop = Crop {
                zoom,
                center_x,
                center_y: 0.5,
            };
            assert!(
                matches!(crop.checked(W, H, 1.0), Err(CoreError::InvalidCrop(_))),
                "{crop:?}"
            );
        }
    }
}
