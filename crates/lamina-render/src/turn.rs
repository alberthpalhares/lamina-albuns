//! A foto girada dentro do quadro (spec AF, regras 2 e 6): a região da foto que cobre o quadro
//! estendido girado e o giro em 1:1 com interpolação bicúbica, depois da redução com Lanczos3.

use lamina_core::{Crop, PixelRegion, Rect, screen_to_photo, turned_box};
use rayon::prelude::*;

use crate::place::{Bled, inside};

/// A região da foto (`photo` em pixels) que cobre o quadro estendido `bled` girado `degrees`,
/// na escala que o recorte dá ao quadro `frame` (mm). Se a foto não tiver sobra para a sangria,
/// o quadro estendido é coberto a partir do mesmo recorte, como sem giro.
pub fn turned_source(
    crop: &Crop,
    (frame, bled): (Rect, &Bled),
    photo: (f64, f64),
    degrees: f64,
) -> PixelRegion {
    let (pw, ph) = photo;
    let (fw, fh) = (frame.width.get(), frame.height.get());
    let base = crop.turned_region(pw, ph, (fw, fh), degrees);
    let per = base.width / turned_box(fw, fh, degrees).0;
    let (bw, bh) = (bled.rect.width.get(), bled.rect.height.get());
    let (tw, th) = turned_box(bw, bh, degrees);
    // O centro do quadro estendido, visto do centro do quadro, levado para a foto.
    let d = (
        bled.rect.x.get() + bw / 2.0 - (frame.x.get() + fw / 2.0),
        bled.rect.y.get() + bh / 2.0 - (frame.y.get() + fh / 2.0),
    );
    let (qx, qy) = screen_to_photo((d.0 * per, d.1 * per), degrees);
    let (cx, cy) = (
        base.x + base.width / 2.0 + qx,
        base.y + base.height / 2.0 + qy,
    );
    let grown = PixelRegion {
        x: cx - tw * per / 2.0,
        y: cy - th * per / 2.0,
        width: tw * per,
        height: th * per,
    };
    let fits = grown.x >= -1e-6
        && grown.y >= -1e-6
        && grown.x + grown.width <= pw + 1e-6
        && grown.y + grown.height <= ph + 1e-6;
    let chosen = if fits {
        grown
    } else {
        crop.turned_region(pw, ph, (bw, bh), degrees)
    };
    inside(chosen, pw, ph)
}

/// O peso da interpolação bicúbica (Catmull-Rom) a `t` pixels de distância.
fn cubic(t: f64) -> f64 {
    let t = t.abs();
    if t < 1.0 {
        1.5 * t * t * t - 2.5 * t * t + 1.0
    } else if t < 2.0 {
        -0.5 * t * t * t + 2.5 * t * t - 4.0 * t + 2.0
    } else {
        0.0
    }
}

/// Uma imagem RGBA em memória (a caixa girada já reduzida).
#[derive(Debug)]
pub struct Plane<'a> {
    pub width: usize,
    pub height: usize,
    pub rgba: &'a [u8],
}

impl Plane<'_> {
    /// O pixel `(x, y)`, preso à borda.
    // Presos entre 0 e o tamanho da imagem antes de virar índice.
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    fn at(&self, x: isize, y: isize) -> [f64; 3] {
        let cx = x.clamp(0, self.width.saturating_sub(1) as isize) as usize;
        let cy = y.clamp(0, self.height.saturating_sub(1) as isize) as usize;
        let i = (cy * self.width + cx) * 4;
        match self.rgba.get(i..i + 3) {
            Some(p) => [f64::from(p[0]), f64::from(p[1]), f64::from(p[2])],
            None => [0.0; 3],
        }
    }

    /// A cor bicúbica no ponto `(x, y)` (centro do pixel 0 em 0,5).
    // Os índices vêm de `floor` e são presos à borda em `at`; as cores, entre 0 e 255.
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    fn sample(&self, x: f64, y: f64) -> [u8; 3] {
        let (fx, fy) = (x - 0.5, y - 0.5);
        let (x0, y0) = (fx.floor(), fy.floor());
        // Quatro pesos por eixo, em vez de dezesseis contas por pixel (spec LV, regra 8): o
        // resultado é o mesmo, a soma segue a mesma ordem.
        let weights = |f: f64, start: f64| [-1.0, 0.0, 1.0, 2.0].map(|n| cubic(f - (start + n)));
        let (wx, wy) = (weights(fx, x0), weights(fy, y0));
        let (ix, iy) = (x0 as isize, y0 as isize);
        let mut sum = [0.0; 3];
        for (j, wy) in (-1..=2).zip(wy) {
            for (i, wx) in (-1..=2).zip(wx) {
                let (w, p) = (wx * wy, self.at(ix + i, iy + j));
                for k in 0..3 {
                    sum[k] += w * p[k];
                }
            }
        }
        sum.map(|v| v.round().clamp(0.0, 255.0) as u8)
    }
}

/// Gira a caixa `box_` (já reduzida para a escala de saída, cobrindo o quadro estendido girado)
/// para dentro de `out_w` × `out_h` pixels: cada pixel de saída lê o ponto da caixa que o giro
/// leva até ele. `px_per_mm` é a escala de saída em cada eixo; `box_mm`, o tamanho da caixa.
pub fn rotate_into(
    box_: &Plane,
    box_mm: (f64, f64),
    (out_w, out_h): (u32, u32),
    px_per_mm: (f64, f64),
    degrees: f64,
) -> Vec<u8> {
    let (bx, by) = (box_.width as f64 / box_mm.0, box_.height as f64 / box_mm.1);
    let (half_w, half_h) = (f64::from(out_w) / 2.0, f64::from(out_h) / 2.0);
    let (box_cx, box_cy) = (box_.width as f64 / 2.0, box_.height as f64 / 2.0);
    let row = out_w as usize * 4;
    let mut out = vec![0u8; row * out_h as usize];
    out.par_chunks_exact_mut(row)
        .enumerate()
        .for_each(|(j, line)| {
            let py = (j as f64 + 0.5 - half_h) / px_per_mm.1;
            for (i, px) in line.as_chunks_mut::<4>().0.iter_mut().enumerate() {
                let px_mm = (i as f64 + 0.5 - half_w) / px_per_mm.0;
                let (qx, qy) = screen_to_photo((px_mm, py), degrees);
                let [r, g, b] = box_.sample(box_cx + qx * bx, box_cy + qy * by);
                *px = [r, g, b, 255];
            }
        });
    out
}
