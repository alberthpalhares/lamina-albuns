//! O tom da foto (spec AF, regra 5): preto e branco, brilho e contraste numa tabela de 256 valores
//! feita só com somas, multiplicações e arredondamento, igual em qualquer máquina (AGENTS, regra
//! 12). Ordem: temperatura e matiz (spec TM, uma tabela por canal) → preto e branco → brilho →
//! contraste. Ajuste neutro não toca na foto.

use lamina_core::PhotoAdjust;

/// A tabela do brilho e do contraste: o nível de saída de cada nível de entrada.
pub fn tone_table(adjust: &PhotoAdjust) -> [u8; 256] {
    let b = f64::from(adjust.brightness) / 100.0;
    let c = f64::from(adjust.contrast) / 100.0;
    let mut table = [0u8; 256];
    for (v, out) in (0u8..=255).zip(table.iter_mut()) {
        let x = f64::from(v) / 255.0;
        // Brilho: B(x) = x + b·x·(1−x).
        let y = x + b * x * (1.0 - x);
        // Contraste: C(y) = y + c·(S(y) − y), com S(y) = y²·(3 − 2y).
        let s = y * y * (3.0 - 2.0 * y);
        let z = y + c * (s - y);
        // Preso entre 0 e 255, então o cast não trunca.
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let level = (z * 255.0).round().clamp(0.0, 255.0) as u8;
        *out = level;
    }
    table
}

/// O cinza do preto e branco: g = (2126·R + 7152·G + 722·B + 5000) / 10000, em inteiros.
pub fn gray(r: u8, g: u8, b: u8) -> u8 {
    let sum = 2126 * u32::from(r) + 7152 * u32::from(g) + 722 * u32::from(b) + 5000;
    // No máximo 255 (os pesos somam 10000), então cabe em u8.
    u8::try_from(sum / 10000).unwrap_or(u8::MAX)
}

/// As tabelas da cor (spec TM, regra 2), uma por canal: o nível de entrada vezes o ganho do
/// canal, arredondado e preso entre 0 e 255. Ganhos: vermelho 1 + 0,25·t, verde 1 − 0,20·m e azul
/// 1 − 0,25·t, com t = temperatura / 100 e m = matiz / 100.
pub fn color_tables(adjust: &PhotoAdjust) -> [[u8; 256]; 3] {
    let t = f64::from(adjust.temperature) / 100.0;
    let m = f64::from(adjust.tint) / 100.0;
    let gains = [1.0 + 0.25 * t, 1.0 - 0.20 * m, 1.0 - 0.25 * t];
    gains.map(|gain| {
        let mut table = [0u8; 256];
        for (v, out) in (0u8..=255).zip(table.iter_mut()) {
            // Preso entre 0 e 255, então o cast não trunca.
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            let level = (f64::from(v) * gain).round().clamp(0.0, 255.0) as u8;
            *out = level;
        }
        table
    })
}

/// Aplica a cor e o tom de `adjust` nos pixels RGBA (o alfa fica). Sem cor nem tom, não mexe em
/// nada.
pub fn apply_tone(rgba: &mut [u8], adjust: &PhotoAdjust) {
    let (color, tone) = (adjust.has_color(), adjust.has_tone());
    if adjust.bw {
        if color {
            let tables = color_tables(adjust);
            apply_tables(rgba, &tables);
        }
        let table = tone_table(adjust);
        for px in rgba.as_chunks_mut::<4>().0 {
            let g = table[usize::from(gray(px[0], px[1], px[2]))];
            *px = [g, g, g, px[3]];
        }
    } else if color || tone {
        // Sem preto e branco, a cor e o tom viram uma tabela por canal e uma passada só (spec LV,
        // regra 8): cada canal só depende dele mesmo, então o resultado é o mesmo das duas passadas.
        let mut tables = if color {
            color_tables(adjust)
        } else {
            [IDENTITY; 3]
        };
        if tone {
            let table = tone_table(adjust);
            for level in tables.iter_mut().flatten() {
                *level = table[usize::from(*level)];
            }
        }
        apply_tables(rgba, &tables);
    }
}

/// Cada nível fica como está.
const IDENTITY: [u8; 256] = {
    let mut table = [0u8; 256];
    let mut v = 0;
    while v < 256 {
        // De 0 a 255: cabe em u8.
        #[allow(clippy::cast_possible_truncation)]
        {
            table[v] = v as u8;
        }
        v += 1;
    }
    table
};

fn apply_tables(rgba: &mut [u8], [r, g, b]: &[[u8; 256]; 3]) {
    for px in rgba.as_chunks_mut::<4>().0 {
        *px = [
            r[usize::from(px[0])],
            g[usize::from(px[1])],
            b[usize::from(px[2])],
            px[3],
        ];
    }
}
