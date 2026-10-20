//! O giro da foto dentro do quadro (spec AF, regras 2 a 4): a caixa que cobre o quadro girado, o
//! fator de cobertura e o recorte que nunca deixa canto vazio. Ângulos em graus, positivos no
//! sentido horário (como a foto aparece na tela).

use super::{Crop, PixelRegion};

/// Largura e altura da caixa que cobre um quadro `w` × `h` girado `degrees` (regra 2):
/// W' = W·|cos θ| + H·|sin θ| e H' = W·|sin θ| + H·|cos θ|.
pub fn turned_box(w: f64, h: f64, degrees: f64) -> (f64, f64) {
    let (s, c) = degrees.to_radians().sin_cos();
    let (s, c) = (s.abs(), c.abs());
    (w * c + h * s, w * s + h * c)
}

/// O fator de cobertura K (regra 2): quanto o giro aproxima a foto `photo` (largura, altura) num
/// quadro `frame` (largura, altura). Com 0°, 1.
pub fn cover_factor(photo: (f64, f64), frame: (f64, f64), degrees: f64) -> f64 {
    let (pw, ph) = photo;
    let (w, h) = frame;
    let (tw, th) = turned_box(w, h, degrees);
    (pw / w).min(ph / h) / (pw / tw).min(ph / th)
}

/// Um vetor da tela (`x`, `y`, com y para baixo) levado para as coordenadas da foto girada
/// `degrees` no sentido horário: gira o vetor no sentido contrário.
pub fn screen_to_photo((x, y): (f64, f64), degrees: f64) -> (f64, f64) {
    let (s, c) = degrees.to_radians().sin_cos();
    (x * c + y * s, -x * s + y * c)
}

impl Crop {
    /// A região da foto (em pixels) que cobre o quadro `frame` (largura, altura em qualquer
    /// unidade) girado `degrees`: a caixa de [`turned_box`] com o zoom e o centro do recorte, sem
    /// sair da foto. Com 0°, é a [`Crop::visible_region`] de sempre.
    pub fn turned_region(
        &self,
        photo_w: f64,
        photo_h: f64,
        frame: (f64, f64),
        degrees: f64,
    ) -> PixelRegion {
        let (tw, th) = turned_box(frame.0, frame.1, degrees);
        self.visible_region(photo_w, photo_h, tw / th)
    }

    /// Arrastar com giro (regra 4): o mouse anda `dx`, `dy` frações do quadro na tela e a foto
    /// acompanha na tela, sem deixar canto vazio.
    pub fn panned_turned(
        &self,
        (photo_w, photo_h): (f64, f64),
        frame: (f64, f64),
        degrees: f64,
        (dx, dy): (f64, f64),
    ) -> Crop {
        let (tw, th) = turned_box(frame.0, frame.1, degrees);
        let region = self.turned_region(photo_w, photo_h, frame, degrees);
        // Pixels da foto por unidade do quadro.
        let per = region.width / tw;
        let (qx, qy) = screen_to_photo((dx * frame.0 * per, dy * frame.1 * per), degrees);
        let moved = Crop {
            center_x: self.center_x - qx / photo_w,
            center_y: self.center_y - qy / photo_h,
            ..*self
        };
        moved.clamped(photo_w, photo_h, tw / th)
    }

    /// O recorte trazido para o que a região girada realmente usa (o centro que não sai da foto).
    pub fn turned_clamped(
        &self,
        (photo_w, photo_h): (f64, f64),
        frame: (f64, f64),
        degrees: f64,
    ) -> Crop {
        let (tw, th) = turned_box(frame.0, frame.1, degrees);
        self.clamped(photo_w, photo_h, tw / th)
    }
}
