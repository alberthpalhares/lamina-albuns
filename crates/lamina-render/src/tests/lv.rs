//! LV-B12 (spec `leveza`): a cor, o tom e o giro saem com os mesmos pixels depois de ficarem mais
//! rápidos. As contas de antes ficam aqui, escritas por extenso, como referência.

use lamina_core::PhotoAdjust;

use crate::turn::{Plane, rotate_into};
use crate::{apply_tone, color_tables, gray, tone_table};

/// Todos os pares de canais que importam, em 4 pixels por linha.
fn pixels() -> Vec<u8> {
    let mut out = Vec::new();
    for v in 0..=255u8 {
        out.extend([v, v.wrapping_mul(7), 255 - v, v / 2 + 1]);
    }
    out
}

/// A conta de antes: uma passada de cor e outra de tom.
fn two_passes(rgba: &mut [u8], adjust: &PhotoAdjust) {
    if adjust.has_color() {
        let [r, g, b] = color_tables(adjust);
        for px in rgba.as_chunks_mut::<4>().0 {
            *px = [
                r[usize::from(px[0])],
                g[usize::from(px[1])],
                b[usize::from(px[2])],
                px[3],
            ];
        }
    }
    if !adjust.has_tone() {
        return;
    }
    let table = tone_table(adjust);
    for px in rgba.as_chunks_mut::<4>().0 {
        if adjust.bw {
            let g = gray(px[0], px[1], px[2]);
            *px = [g, g, g, px[3]];
        }
        for ch in &mut px[..3] {
            *ch = table[usize::from(*ch)];
        }
    }
}

#[test]
fn lv_b12_tom_igual_ao_das_duas_passadas() {
    for (brightness, contrast, temperature, tint, bw) in [
        (0, 0, 0, 0, false),
        (12, -8, 0, 0, false),
        (0, 0, 40, -30, false),
        (-30, 25, -60, 15, false),
        (0, 0, 0, 0, true),
        (10, 10, 30, 30, true),
    ] {
        let adjust = PhotoAdjust {
            brightness,
            contrast,
            temperature,
            tint,
            bw,
            ..PhotoAdjust::default()
        };
        let (mut now, mut before) = (pixels(), pixels());
        apply_tone(&mut now, &adjust);
        two_passes(&mut before, &adjust);
        assert_eq!(now, before, "{adjust:?}");
    }
}

/// A conta de antes do giro, com dezesseis pesos por pixel.
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn sample_before(plane: &Plane, x: f64, y: f64) -> [u8; 3] {
    let cubic = |t: f64| {
        let t = t.abs();
        if t < 1.0 {
            1.5 * t * t * t - 2.5 * t * t + 1.0
        } else if t < 2.0 {
            -0.5 * t * t * t + 2.5 * t * t - 4.0 * t + 2.0
        } else {
            0.0
        }
    };
    let (fx, fy) = (x - 0.5, y - 0.5);
    let (x0, y0) = (fx.floor(), fy.floor());
    let mut sum = [0.0; 3];
    for j in -1..=2 {
        let wy = cubic(fy - (y0 + f64::from(j)));
        for i in -1..=2 {
            let w = cubic(fx - (x0 + f64::from(i))) * wy;
            let cx = (x0 as isize + i as isize).clamp(0, plane.width as isize - 1) as usize;
            let cy = (y0 as isize + j as isize).clamp(0, plane.height as isize - 1) as usize;
            let at = (cy * plane.width + cx) * 4;
            for (total, level) in sum.iter_mut().zip(&plane.rgba[at..at + 3]) {
                *total += w * f64::from(*level);
            }
        }
    }
    sum.map(|v| v.round().clamp(0.0, 255.0) as u8)
}

#[test]
fn lv_b12_giro_igual_ao_dos_dezesseis_pesos() {
    let (w, h) = (40usize, 30usize);
    let mut rgba = Vec::with_capacity(w * h * 4);
    for y in 0..h {
        for x in 0..w {
            let v = |n: usize| u8::try_from((x * 7 + y * 13 + n * 29) % 256).unwrap();
            rgba.extend([v(0), v(1), v(2), 255]);
        }
    }
    let plane = Plane {
        width: w,
        height: h,
        rgba: &rgba,
    };
    let (out_w, out_h) = (24u32, 18u32);
    let degrees = 12.5;
    let box_mm = (40.0, 30.0);
    let now = rotate_into(&plane, box_mm, (out_w, out_h), (1.0, 1.0), degrees);
    for j in 0..out_h {
        for i in 0..out_w {
            let px = (
                f64::from(i) + 0.5 - f64::from(out_w) / 2.0,
                f64::from(j) + 0.5 - f64::from(out_h) / 2.0,
            );
            let (qx, qy) = lamina_core::screen_to_photo(px, degrees);
            let before = sample_before(&plane, 20.0 + qx, 15.0 + qy);
            let at = ((j * out_w + i) * 4) as usize;
            assert_eq!(now[at..at + 3], before, "pixel {i},{j}");
        }
    }
}
