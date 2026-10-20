//! O seletor de cor das telas (spec NP, etapa 2): o quadro de cores em que o fotógrafo clica.
//! Na horizontal, a matiz (vermelho → amarelo → verde → ciano → azul → magenta → vermelho); na
//! vertical, do branco no topo à cor pura no meio e ao preto embaixo. Assim cabem os tons claros
//! (creme, rosa-bebê) e os escuros (vinho, marinho) que os álbuns usam.

use lamina_core::Rgb;

/// A cor no ponto (`x`, `y`) do quadro, os dois de 0 a 1 (fora disso, a borda mais perto).
pub fn picker_color(x: f64, y: f64) -> Rgb {
    let (x, y) = (x.clamp(0.0, 1.0), y.clamp(0.0, 1.0));
    let pure = hue(x);
    let mix = |a: f64, b: f64, t: f64| a + (b - a) * t;
    let channel = |c: f64| {
        let v = if y <= 0.5 {
            mix(1.0, c, y * 2.0)
        } else {
            mix(c, 0.0, (y - 0.5) * 2.0)
        };
        byte(v)
    };
    Rgb {
        r: channel(pure.0),
        g: channel(pure.1),
        b: channel(pure.2),
    }
}

/// A cor pura da matiz `h` (0 a 1), em frações de 0 a 1.
// O setor vai de 0 a 5.
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn hue(h: f64) -> (f64, f64, f64) {
    let sector = (h * 6.0).min(5.999_999);
    let f = sector.fract();
    match sector as u8 {
        0 => (1.0, f, 0.0),
        1 => (1.0 - f, 1.0, 0.0),
        2 => (0.0, 1.0, f),
        3 => (0.0, 1.0 - f, 1.0),
        4 => (f, 0.0, 1.0),
        _ => (1.0, 0.0, 1.0 - f),
    }
}

// O valor já está entre 0 e 255 depois do `clamp`.
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn byte(v: f64) -> u8 {
    (v.clamp(0.0, 1.0) * 255.0).round() as u8
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quadro_de_cores_vai_do_branco_a_cor_pura_e_ao_preto() {
        let rgb = |r, g, b| Rgb { r, g, b };
        assert_eq!(picker_color(0.0, 0.0), rgb(255, 255, 255), "topo: branco");
        assert_eq!(picker_color(0.3, 1.0), rgb(0, 0, 0), "embaixo: preto");
        assert_eq!(
            picker_color(0.0, 0.5),
            rgb(255, 0, 0),
            "meio: vermelho puro"
        );
        assert_eq!(picker_color(1.0 / 3.0, 0.5), rgb(0, 255, 0));
        assert_eq!(picker_color(2.0 / 3.0, 0.5), rgb(0, 0, 255));
        assert_eq!(picker_color(0.0, 0.25), rgb(255, 128, 128), "tom claro");
        assert_eq!(picker_color(2.0 / 3.0, 0.75), rgb(0, 0, 128), "tom escuro");
        assert_eq!(
            picker_color(-1.0, 2.0),
            rgb(0, 0, 0),
            "fora do quadro: a borda"
        );
    }
}
