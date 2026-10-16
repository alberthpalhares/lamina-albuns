//! Qualidade de impressão (AGENTS.md, regra 14; decisão do dono, 2026-10-15): JPEG com qualidade
//! 95 por padrão e cor em resolução total (4:4:4, o que o codificador do `image` grava),
//! imperceptível em fotos. As travas conferem a configuração gravada
//! no arquivo e a fidelidade de um pior caso.

use image::{Rgba, RgbaImage};
use lamina_core::Photo;

use super::*;
use crate::ExportOptions;

/// Fidelidade (PSNR, dB) entre duas imagens do mesmo tamanho, nos canais de cor.
fn psnr(a: &image::RgbImage, b: &image::RgbImage) -> f64 {
    let pixels = a.as_raw().iter().zip(b.as_raw());
    let sum: f64 = pixels
        .map(|(x, y)| (f64::from(*x) - f64::from(*y)).powi(2))
        .sum();
    let mse = sum / a.as_raw().len() as f64;
    10.0 * (255.0f64.powi(2) / mse).log10()
}

/// Pior caso para o JPEG: granulação forte e linhas finas coloridas.
fn worst_case(size: u32) -> RgbaImage {
    let mut seed: u32 = 7;
    RgbaImage::from_fn(size, size, |x, y| {
        seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        let grain = i32::try_from(seed >> 27).unwrap() - 16;
        let line = if x % 24 < 2 {
            [180, -60, -60]
        } else if y % 31 < 2 {
            [-60, -60, 180]
        } else {
            [0, 0, 0]
        };
        let ramp = |v: u32| i32::try_from(v * 200 / size).unwrap();
        let base = [ramp(x), ramp(y), 120];
        let c = |i: usize| u8::try_from((base[i] + line[i] + grain).clamp(0, 255)).unwrap();
        Rgba([c(0), c(1), c(2), 255])
    })
}

/// Exporta uma lâmina com `photo` cobrindo a página esquerda e devolve (bytes do JPEG,
/// renderização, JPEG lido de volta).
fn export_one(photo: RgbaImage) -> (Vec<u8>, image::RgbImage, image::RgbImage) {
    let mut album = Album::new(spec(), 1);
    let p = album.photo("foto.jpg", photo);
    album.photo_frame(0, p, [0.0, 0.0, 0.5, 1.0]);
    let dir = tempfile::tempdir().unwrap();
    let report = album.export(dir.path(), &ExportOptions::default()).unwrap();
    let source = |ph: &Photo, _: (u32, u32)| {
        album
            .images
            .get(&ph.path)
            .cloned()
            .ok_or_else(|| "sumiu".to_owned())
    };
    let rendered = lamina_render::render_spread(
        &album.project,
        0,
        lamina_render::RenderTarget::Print,
        &source,
    )
    .unwrap();
    let bytes = std::fs::read(&report.files[0]).unwrap();
    let exported = image::load_from_memory(&bytes).unwrap().into_rgb8();
    (
        bytes,
        image::DynamicImage::ImageRgba8(rendered).into_rgb8(),
        exported,
    )
}

/// Fatores de amostragem (H, V) de cada componente, lidos do cabeçalho SOF do JPEG.
fn sampling(jpeg: &[u8]) -> Vec<(u8, u8)> {
    let sof = jpeg.windows(2).position(|w| w == [0xFF, 0xC0]).unwrap();
    let count = usize::from(jpeg[sof + 9]);
    (0..count)
        .map(|k| {
            let f = jpeg[sof + 11 + 3 * k];
            (f >> 4, f & 15)
        })
        .collect()
}

/// Qualidade estimada pela primeira tabela de quantização (a do brilho), comparando com as
/// tabelas padrão escaladas da libjpeg.
fn estimated_quality(jpeg: &[u8]) -> u8 {
    const BASE: [u32; 8] = [16, 11, 10, 16, 24, 40, 51, 61];
    let dqt = jpeg.windows(2).position(|w| w == [0xFF, 0xDB]).unwrap();
    // Os 8 primeiros valores em zigue-zague: posições (0,0) (0,1) (1,0) (2,0) (1,1) (0,2) (0,3) (1,2).
    let zz = &jpeg[dqt + 5..dqt + 5 + 64];
    let row: [u32; 8] = [zz[0], zz[1], zz[5], zz[6], zz[14], zz[15], zz[27], zz[28]].map(u32::from);
    (1..=100u8)
        .min_by_key(|&q| {
            let q = u32::from(q);
            let scale = if q < 50 { 5000 / q } else { 200 - 2 * q };
            BASE.iter()
                .zip(row)
                .map(|(b, v)| ((b * scale + 50) / 100).clamp(1, 255).abs_diff(v))
                .sum::<u32>()
        })
        .unwrap()
}

/// Regra 14: a configuração gravada no arquivo é cor em resolução total (4:4:4; nunca 4:2:0) e
/// qualidade ≥ 95. A documentação do `image` fala em 4:2:2, mas o arquivo diz 4:4:4: esta trava
/// confere o arquivo, não a documentação.
#[test]
fn exportacao_padrao_usa_4_4_4_e_qualidade_95() {
    let (bytes, _, _) = export_one(worst_case(600));
    assert_eq!(sampling(&bytes), [(1, 1), (1, 1), (1, 1)], "esperava 4:4:4");
    assert!(
        estimated_quality(&bytes) >= 95,
        "qualidade {}",
        estimated_quality(&bytes)
    );
}

/// Regra 14: no pior caso (granulação forte e linhas finas coloridas, que nenhuma foto real
/// tem), a fidelidade dentro da foto fica em ~39,6 dB com a qualidade 95; a 90 cai para ~35 dB.
/// O limite de 38 dB barra qualquer padrão abaixo da 95. Em fotos reais, a 95 dá ~48 dB.
#[test]
fn exportacao_padrao_tem_qualidade_de_impressao() {
    let (_, rendered, exported) = export_one(worst_case(3600));
    // Só a página esquerda, onde está a foto: o fundo branco liso esconderia a perda.
    let half = rendered.width() / 2 - 20;
    let crop =
        |img: &image::RgbImage| image::imageops::crop_imm(img, 0, 0, half, img.height()).to_image();
    let db = psnr(&crop(&rendered), &crop(&exported));
    assert!(db >= 38.0, "fidelidade {db:.1} dB abaixo de 38 dB");
}
