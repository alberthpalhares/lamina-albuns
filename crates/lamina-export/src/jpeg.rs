//! JPEG de impressão (spec EX, regra 2): RGB de 8 bits, densidade em DPI e perfil sRGB embutido.

use std::sync::OnceLock;

use image::codecs::jpeg::{JpegEncoder, PixelDensity};
use image::{DynamicImage, ExtendedColorType, ImageEncoder, RgbaImage};

/// Data fixa gravada no cabeçalho do perfil (2026-01-01 00:00:00): ano, mês, dia, hora,
/// minuto e segundo, em u16 big-endian (ICC.1, bytes 24 a 35).
const PROFILE_DATE: [u16; 6] = [2026, 1, 1, 0, 0, 0];

/// Perfil ICC sRGB, gerado uma vez pelo `moxcms` (ADR 0007).
pub fn srgb_profile() -> Result<&'static [u8], String> {
    static PROFILE: OnceLock<Result<Vec<u8>, String>> = OnceLock::new();
    PROFILE
        .get_or_init(build_profile)
        .as_ref()
        .map(Vec::as_slice)
        .map_err(Clone::clone)
}

/// O `moxcms` grava a data e a hora atuais no perfil; com a data fixa, o mesmo álbum gera os
/// mesmos arquivos em qualquer dia (spec EX, regra 10). O identificador do perfil (MD5 dos
/// bytes, opcional no ICC) é zerado, porque mudaria com a data.
fn build_profile() -> Result<Vec<u8>, String> {
    let mut icc = moxcms::ColorProfile::new_srgb()
        .encode()
        .map_err(|e| e.to_string())?;
    if icc.len() < 128 {
        return Err("perfil ICC menor que o cabeçalho".to_owned());
    }
    for (i, value) in PROFILE_DATE.iter().enumerate() {
        icc[24 + 2 * i..26 + 2 * i].copy_from_slice(&value.to_be_bytes());
    }
    icc[84..100].fill(0);
    Ok(icc)
}

/// Codifica a imagem com qualidade `quality` (1 a 100) e densidade `dpi`.
pub fn encode(image: RgbaImage, quality: u8, dpi: u32) -> Result<Vec<u8>, String> {
    let rgb = DynamicImage::ImageRgba8(image).into_rgb8();
    let mut bytes = Vec::new();
    let mut encoder = JpegEncoder::new_with_quality(&mut bytes, quality);
    let density = u16::try_from(dpi).unwrap_or(u16::MAX);
    encoder.set_pixel_density(PixelDensity::dpi(density));
    encoder
        .set_icc_profile(srgb_profile()?.to_vec())
        .map_err(|e| e.to_string())?;
    encoder
        .write_image(
            rgb.as_raw(),
            rgb.width(),
            rgb.height(),
            ExtendedColorType::Rgb8,
        )
        .map_err(|e| e.to_string())?;
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn perfil_srgb_e_um_icc_valido() {
        let icc = srgb_profile().unwrap();
        assert_eq!(&icc[36..40], b"acsp", "assinatura de perfil ICC");
        // O próprio moxcms lê o perfil de volta (falha se estiver malformado).
        let profile = moxcms::ColorProfile::new_from_slice(icc).unwrap();
        assert_eq!(profile.color_space, moxcms::DataColorSpace::Rgb);
        // É sRGB: mesmas primárias e ponto branco do sRGB de referência (auditoria do Marco 1,
        // M10: antes só a ida e volta era conferida).
        let srgb = moxcms::ColorProfile::new_srgb();
        assert_eq!(profile.red_colorant, srgb.red_colorant);
        assert_eq!(profile.green_colorant, srgb.green_colorant);
        assert_eq!(profile.blue_colorant, srgb.blue_colorant);
        assert_eq!(profile.white_point, srgb.white_point);
    }

    /// Defeito achado no CI: o moxcms grava a hora atual, e dois perfis gerados em segundos
    /// diferentes saíam diferentes (arquivos exportados mudariam de um dia para o outro).
    #[test]
    fn perfil_tem_data_fixa_e_sai_sempre_igual() {
        let icc = build_profile().unwrap();
        assert_eq!(&icc[24..26], &2026u16.to_be_bytes());
        assert_eq!(&icc[26..36], &[0, 1, 0, 1, 0, 0, 0, 0, 0, 0]);
        assert!(icc[84..100].iter().all(|b| *b == 0), "identificador zerado");
        std::thread::sleep(std::time::Duration::from_millis(1100));
        assert_eq!(build_profile().unwrap(), icc, "mudou depois de um segundo");
    }
}
