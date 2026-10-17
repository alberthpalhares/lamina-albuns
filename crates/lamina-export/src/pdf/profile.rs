//! Perfis de cor do PDF de impressão (spec PD, regra 4; ADR 0007, adendo do PD). Aprovados no
//! Preflight PDF/X-4 do Acrobat pelo protótipo (2026-10-16):
//!
//! - o da **imagem** é o sRGB do `moxcms` sem a etiqueta `cicp` e marcado como ICC 4.2 (o 4.3/4.4
//!   que o `moxcms` grava é mais novo que o PDF/X-4);
//! - o **perfil de saída** é o mesmo sRGB como perfil de impressora (`prtr`), com PCS Lab e tabelas
//!   lut16 de 33 pontos (A2B: RGB → Lab; B2A: Lab → RGB) calculadas das fórmulas do sRGB. Só com
//!   a matriz o Preflight passava, mas o Acrobat dizia "espaço de cor inválido" ao desenhar.

use std::sync::OnceLock;

use moxcms::{
    ColorProfile, DataColorSpace, LutDataType, LutStore, LutType, LutWarehouse, Matrix3d,
    ProfileClass, ProfileText,
};

use crate::jpeg::fix_header;

/// Pontos da grade 3D das tabelas.
const GRID: usize = 33;
/// Branco D50 do PCS.
const D50: [f64; 3] = [0.9642, 1.0, 0.8249];

/// O sRGB da imagem (classe `mntr`).
pub fn image_profile() -> Result<&'static [u8], String> {
    static PROFILE: OnceLock<Result<Vec<u8>, String>> = OnceLock::new();
    PROFILE
        .get_or_init(|| {
            let mut profile = ColorProfile::new_srgb();
            profile.cicp = None;
            encode(&profile)
        })
        .as_ref()
        .map(Vec::as_slice)
        .map_err(Clone::clone)
}

/// O perfil de saída (classe `prtr`, PCS Lab, tabelas lut16).
pub fn output_profile() -> Result<&'static [u8], String> {
    static PROFILE: OnceLock<Result<Vec<u8>, String>> = OnceLock::new();
    PROFILE
        .get_or_init(build_output)
        .as_ref()
        .map(Vec::as_slice)
        .map_err(Clone::clone)
}

/// Descrição do perfil de saída, também usada no `OutputIntent`.
pub const OUTPUT_CONDITION: &str = "sRGB IEC61966-2.1";

fn encode(profile: &ColorProfile) -> Result<Vec<u8>, String> {
    let icc = profile.encode().map_err(|e| e.to_string())?;
    let mut icc = fix_header(icc)?;
    // Versão 4.2 (bytes 8 a 11): a do PDF 1.7.
    icc[8..12].copy_from_slice(&[4, 0x20, 0, 0]);
    Ok(icc)
}

fn build_output() -> Result<Vec<u8>, String> {
    let base = ColorProfile::new_srgb();
    let to_xyz = base.colorant_matrix();
    let to_rgb = invert(&to_xyz).ok_or("matriz do sRGB sem inversa")?;
    let (to_lab, from_lab) = tables(&to_xyz, &to_rgb);
    let mut profile = ColorProfile::new_srgb();
    profile.profile_class = ProfileClass::OutputDevice;
    profile.pcs = DataColorSpace::Lab;
    profile.cicp = None;
    profile.red_trc = None;
    profile.green_trc = None;
    profile.blue_trc = None;
    profile.description = Some(ProfileText::PlainString(format!(
        "{OUTPUT_CONDITION} para impressao (Lamina)"
    )));
    let (a2b, b2a) = (lut(to_lab), lut(from_lab));
    profile.lut_a_to_b_perceptual = Some(a2b.clone());
    profile.lut_a_to_b_colorimetric = Some(a2b);
    profile.lut_b_to_a_perceptual = Some(b2a.clone());
    profile.lut_b_to_a_colorimetric = Some(b2a);
    encode(&profile)
}

/// As duas grades: RGB → Lab e Lab → RGB, na codificação lut16 do Lab (L 0..100 → 0..0xFF00;
/// a e b: (valor + 128) × 256). A primeira entrada varia mais devagar (ICC.1).
fn tables(to_xyz: &Matrix3d, to_rgb: &Matrix3d) -> (Vec<u16>, Vec<u16>) {
    let points = GRID * GRID * GRID * 3;
    let (mut to_lab, mut from_lab) = (Vec::with_capacity(points), Vec::with_capacity(points));
    let step = |i: usize| as_f64(i) / as_f64(GRID - 1);
    for i in 0..GRID {
        for j in 0..GRID {
            for k in 0..GRID {
                let u = [step(i), step(j), step(k)];
                let [l, a, b] = lab_of(mul(to_xyz, u.map(decode)));
                to_lab.extend([to_u16(l / 100.0 * 65280.0), lab_ab(a), lab_ab(b)]);
                let lab = [
                    u[0] * 65535.0 / 65280.0 * 100.0,
                    u[1] * 65535.0 / 256.0 - 128.0,
                    u[2] * 65535.0 / 256.0 - 128.0,
                ];
                let rgb = mul(to_rgb, xyz_of(lab)).map(encode_trc);
                from_lab.extend(rgb.map(|c| to_u16(c * 65535.0)));
            }
        }
    }
    (to_lab, from_lab)
}

fn lut(clut: Vec<u16>) -> LutWarehouse {
    let identity: Vec<u16> = (0..3)
        .flat_map(|_| (0..256u32).map(|i| to_u16(f64::from(i) * 65535.0 / 255.0)))
        .collect();
    LutWarehouse::Lut(LutDataType {
        num_input_channels: 3,
        num_output_channels: 3,
        num_clut_grid_points: 33,
        matrix: Matrix3d {
            v: [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
        },
        num_input_table_entries: 256,
        num_output_table_entries: 256,
        input_table: LutStore::Store16(identity.clone()),
        clut_table: LutStore::Store16(clut),
        output_table: LutStore::Store16(identity),
        lut_type: LutType::Lut16,
    })
}

/// Valor codificado do sRGB → linear.
fn decode(v: f64) -> f64 {
    if v <= 0.040_45 {
        v / 12.92
    } else {
        ((v + 0.055) / 1.055).powf(2.4)
    }
}

/// Linear → valor codificado do sRGB.
fn encode_trc(l: f64) -> f64 {
    let l = l.clamp(0.0, 1.0);
    if l <= 0.003_130_8 {
        12.92 * l
    } else {
        1.055 * l.powf(1.0 / 2.4) - 0.055
    }
}

const EPSILON: f64 = 216.0 / 24389.0;
const KAPPA: f64 = 24389.0 / 27.0;

fn lab_of(xyz: [f64; 3]) -> [f64; 3] {
    let f = |t: f64| {
        if t > EPSILON {
            t.cbrt()
        } else {
            (KAPPA * t + 16.0) / 116.0
        }
    };
    let [fx, fy, fz] = [0, 1, 2].map(|i| f(xyz[i] / D50[i]));
    [116.0 * fy - 16.0, 500.0 * (fx - fy), 200.0 * (fy - fz)]
}

fn xyz_of([l, a, b]: [f64; 3]) -> [f64; 3] {
    let fy = (l + 16.0) / 116.0;
    let finv = |f: f64| {
        let cube = f * f * f;
        if cube > EPSILON {
            cube
        } else {
            (116.0 * f - 16.0) / KAPPA
        }
    };
    [
        D50[0] * finv(fy + a / 500.0),
        D50[1] * finv(fy),
        D50[2] * finv(fy - b / 200.0),
    ]
}

fn lab_ab(v: f64) -> u16 {
    to_u16((v + 128.0) * 256.0)
}

fn mul(m: &Matrix3d, v: [f64; 3]) -> [f64; 3] {
    [0, 1, 2].map(|r| m.v[r][0] * v[0] + m.v[r][1] * v[1] + m.v[r][2] * v[2])
}

fn invert(m: &Matrix3d) -> Option<Matrix3d> {
    let a = m.v;
    let cofactor = |r: usize, k: usize| {
        let (r1, r2, k1, k2) = ((r + 1) % 3, (r + 2) % 3, (k + 1) % 3, (k + 2) % 3);
        a[r1][k1] * a[r2][k2] - a[r1][k2] * a[r2][k1]
    };
    let det: f64 = (0..3).map(|k| a[0][k] * cofactor(0, k)).sum();
    if det.abs() < 1e-12 {
        return None;
    }
    let mut v = [[0.0; 3]; 3];
    for (i, row) in v.iter_mut().enumerate() {
        for (j, cell) in row.iter_mut().enumerate() {
            *cell = cofactor(j, i) / det;
        }
    }
    Some(Matrix3d { v })
}

fn as_f64(i: usize) -> f64 {
    // Índices da grade (no máximo 32): cabem sem perda.
    #[allow(clippy::cast_precision_loss)]
    let v = i as f64;
    v
}

fn to_u16(v: f64) -> u16 {
    // Preso entre 0 e 65535 antes do cast.
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let n = v.round().clamp(0.0, 65535.0) as u16;
    n
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn perfis_na_versao_4_2_e_nas_classes_certas() {
        let image = image_profile().unwrap();
        let output = output_profile().unwrap();
        assert_eq!(&image[8..12], &[4, 0x20, 0, 0]);
        assert_eq!(&output[8..12], &[4, 0x20, 0, 0]);
        assert_eq!(&image[12..16], b"mntr");
        assert_eq!(&output[12..16], b"prtr");
        assert_eq!(&output[20..24], b"Lab ", "PCS Lab");
        // As tabelas de um perfil de impressora.
        for tag in [b"A2B0", b"B2A0", b"A2B1", b"B2A1"] {
            assert!(output.windows(4).any(|w| w == tag), "{tag:?}");
        }
        assert!(!image.windows(4).any(|w| w == b"cicp"));
    }

    #[test]
    fn lab_ida_e_volta() {
        for xyz in [[0.9642, 1.0, 0.8249], [0.2, 0.3, 0.1], [0.0, 0.0, 0.0]] {
            let back = xyz_of(lab_of(xyz));
            for i in 0..3 {
                assert!((back[i] - xyz[i]).abs() < 1e-9, "{xyz:?} → {back:?}");
            }
        }
        let white = lab_of(D50);
        assert!((white[0] - 100.0).abs() < 1e-9 && white[1].abs() < 1e-9);
    }
}
