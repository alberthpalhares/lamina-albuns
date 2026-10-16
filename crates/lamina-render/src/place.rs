//! Onde cada quadro cai na tela, em pixels, e que pedaço da foto ele mostra (spec RD, regras 2,
//! 4 e 5).

use lamina_core::{AlbumSpec, Crop, PixelRegion, Rect};

/// Tolerância para dizer que um quadro encosta no corte, em mm.
const TOUCH_MM: f64 = 0.1;

/// Escala da tela: pixels por mm em cada eixo.
#[derive(Debug, Clone, Copy)]
pub struct Scale {
    pub x: f64,
    pub y: f64,
}

/// Retângulo em pixels inteiros da tela: `[x0, x1)` × `[y0, y1)`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PixelBox {
    pub x0: u32,
    pub y0: u32,
    pub x1: u32,
    pub y1: u32,
}

impl PixelBox {
    pub fn width(&self) -> u32 {
        self.x1 - self.x0
    }

    pub fn height(&self) -> u32 {
        self.y1 - self.y0
    }
}

/// O quadro já estendido pela sangria (em mm) e o quanto cresceu de cada lado.
#[derive(Debug, Clone, Copy)]
pub struct Bled {
    pub rect: Rect,
    /// Crescimento em mm: esquerda, cima, direita, baixo.
    pub grow: [f64; 4],
}

/// Estende até a borda da tela os lados do quadro que encostam no corte (regra 5).
pub fn bleed(spec: &AlbumSpec, rect: Rect) -> Bled {
    let (trim, canvas) = (spec.trim_rect(), spec.canvas_rect());
    let touches = |a: f64, b: f64| (a - b).abs() < TOUCH_MM;
    let left = if touches(rect.x.get(), trim.x.get()) {
        canvas.x.get()
    } else {
        rect.x.get()
    };
    let top = if touches(rect.y.get(), trim.y.get()) {
        canvas.y.get()
    } else {
        rect.y.get()
    };
    let right = if touches(rect.right().get(), trim.right().get()) {
        canvas.right().get()
    } else {
        rect.right().get()
    };
    let bottom = if touches(rect.bottom().get(), trim.bottom().get()) {
        canvas.bottom().get()
    } else {
        rect.bottom().get()
    };
    let grow = [
        rect.x.get() - left,
        rect.y.get() - top,
        right - rect.right().get(),
        bottom - rect.bottom().get(),
    ];
    let rect = Rect::new(
        lamina_core::Mm(left),
        lamina_core::Mm(top),
        lamina_core::Mm(right - left),
        lamina_core::Mm(bottom - top),
    );
    Bled { rect, grow }
}

/// Pixels da tela cobertos por `rect` (mm), com bordas arredondadas e presas à tela (regra 4).
/// `None` se o quadro some (menos de 1 pixel).
pub fn pixels(rect: Rect, scale: Scale, size: (u32, u32)) -> Option<PixelBox> {
    let to_px = |mm: f64, s: f64, max: u32| {
        let v = (mm * s).round().clamp(0.0, f64::from(max));
        // Preso entre 0 e o tamanho da tela, então cabe em u32.
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let px = v as u32;
        px
    };
    let b = PixelBox {
        x0: to_px(rect.x.get(), scale.x, size.0),
        y0: to_px(rect.y.get(), scale.y, size.1),
        x1: to_px(rect.right().get(), scale.x, size.0),
        y1: to_px(rect.bottom().get(), scale.y, size.1),
    };
    (b.x1 > b.x0 && b.y1 > b.y0).then_some(b)
}

/// Pedaço da foto (`photo_w` × `photo_h` pixels) mostrado pelo quadro `frame` (mm) estendido
/// como `bled`. O pedaço do recorte cresce na mesma proporção da sangria; se a foto não tiver
/// sobra para isso, o quadro estendido é coberto a partir do mesmo recorte.
pub fn region(crop: &Crop, frame: Rect, bled: &Bled, photo: (f64, f64)) -> PixelRegion {
    let (pw, ph) = photo;
    let aspect = frame.width.get() / frame.height.get();
    let base = crop.visible_region(pw, ph, aspect);
    let (per_x, per_y) = (
        base.width / frame.width.get(),
        base.height / frame.height.get(),
    );
    let [left, top, right, bottom] = bled.grow;
    let grown = PixelRegion {
        x: base.x - left * per_x,
        y: base.y - top * per_y,
        width: base.width + (left + right) * per_x,
        height: base.height + (top + bottom) * per_y,
    };
    let fits = grown.x >= -1e-6
        && grown.y >= -1e-6
        && grown.x + grown.width <= pw + 1e-6
        && grown.y + grown.height <= ph + 1e-6;
    let chosen = if fits {
        grown
    } else {
        let bled_aspect = bled.rect.width.get() / bled.rect.height.get();
        crop.visible_region(pw, ph, bled_aspect)
    };
    inside(chosen, pw, ph)
}

/// Encaixa a região na foto: as contas em ponto flutuante podem passar da borda por um fio, e
/// a redução recusa qualquer pixel de fora.
fn inside(r: PixelRegion, pw: f64, ph: f64) -> PixelRegion {
    let x = r.x.clamp(0.0, pw);
    let y = r.y.clamp(0.0, ph);
    PixelRegion {
        x,
        y,
        width: r.width.min(pw - x),
        height: r.height.min(ph - y),
    }
}

#[cfg(test)]
mod tests {
    use lamina_core::{Dpi, Mm};

    use super::*;

    fn spec() -> AlbumSpec {
        AlbumSpec::new(Mm::from_cm(30.0), Mm::from_cm(30.0), Dpi::new(300).unwrap())
            .unwrap()
            .with_bleed(Mm(3.0))
            .unwrap()
    }

    #[test]
    fn quadro_no_meio_nao_cresce() {
        let r = Rect::new(Mm(50.0), Mm(50.0), Mm(100.0), Mm(80.0));
        let b = bleed(&spec(), r);
        assert_eq!(b.rect, r);
        assert_eq!(b.grow, [0.0; 4]);
    }

    #[test]
    fn pagina_esquerda_cresce_por_tres_lados() {
        let s = spec();
        let b = bleed(&s, s.page_rect(lamina_core::PageSide::Left));
        assert_eq!(b.grow, [3.0, 3.0, 0.0, 3.0]);
        assert_eq!(b.rect.x, Mm(0.0));
    }

    #[test]
    fn vizinhos_nao_se_sobrepoem_depois_de_arredondar() {
        let scale = Scale {
            x: 1.0 / 3.0,
            y: 1.0 / 3.0,
        };
        let a = Rect::new(Mm(10.0), Mm(10.0), Mm(100.5), Mm(50.0));
        let b = Rect::new(Mm(110.5), Mm(10.0), Mm(100.0), Mm(50.0));
        let (pa, pb) = (
            pixels(a, scale, (1000, 1000)).unwrap(),
            pixels(b, scale, (1000, 1000)).unwrap(),
        );
        assert!(pa.x1 <= pb.x0, "{pa:?} {pb:?}");
    }

    #[test]
    fn quadro_menor_que_um_pixel_some() {
        let r = Rect::new(Mm(10.0), Mm(10.0), Mm(0.1), Mm(0.1));
        assert_eq!(pixels(r, Scale { x: 1.0, y: 1.0 }, (100, 100)), None);
    }

    #[test]
    fn regiao_nunca_passa_da_borda_da_foto() {
        // Zoom 4 com o centro na borda direita/de baixo: a conta pode passar por um fio.
        let crop = Crop {
            zoom: 4.0,
            center_x: 1.0,
            center_y: 1.0,
        };
        let frame = Rect::new(Mm(50.0), Mm(50.0), Mm(70.0), Mm(30.0));
        let bled = bleed(&spec(), frame);
        for (pw, ph) in [(6000.0, 4000.0), (4321.0, 2879.0), (1001.0, 999.0)] {
            let r = region(&crop, frame, &bled, (pw, ph));
            assert!(r.x >= 0.0 && r.y >= 0.0, "{r:?}");
            assert!(
                r.x + r.width <= pw && r.y + r.height <= ph,
                "{r:?} em {pw} × {ph}"
            );
        }
    }

    #[test]
    fn sem_sobra_o_quadro_estendido_e_coberto() {
        // Foto 3:2 num quadro 3:2 encostado à esquerda: não há sobra para crescer 3 mm.
        let s = spec();
        let frame = Rect::new(s.trim_rect().x, Mm(60.0), Mm(150.0), Mm(100.0));
        let bled = bleed(&s, frame);
        assert_eq!(bled.grow, [3.0, 0.0, 0.0, 0.0]);
        let r = region(&Crop::default(), frame, &bled, (3000.0, 2000.0));
        let aspect = bled.rect.width.get() / bled.rect.height.get();
        assert!((r.width / r.height - aspect).abs() < 1e-9, "{r:?}");
        assert!(
            r.x >= 0.0 && r.x + r.width <= 3000.0 && r.height <= 2000.0,
            "{r:?}"
        );
    }

    #[test]
    fn regiao_cresce_com_a_sangria_quando_a_foto_tem_sobra() {
        // Foto 3:2 num quadro quadrado: sobra largura, falta altura para crescer em cima.
        let frame = Rect::new(Mm(3.0), Mm(50.0), Mm(100.0), Mm(100.0));
        let bled = Bled {
            rect: Rect::new(Mm(0.0), Mm(50.0), Mm(103.0), Mm(100.0)),
            grow: [3.0, 0.0, 0.0, 0.0],
        };
        let r = region(&Crop::default(), frame, &bled, (3000.0, 2000.0));
        // Base: 2000 × 2000 a partir de x = 500; cresce 3 mm × 20 px/mm = 60 px à esquerda.
        assert!(
            (r.x - 440.0).abs() < 1e-9 && (r.width - 2060.0).abs() < 1e-9,
            "{r:?}"
        );
    }
}
