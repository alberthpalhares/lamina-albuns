//! Cor das fotos (spec CT, regras 1 e 2): o perfil ICC embutido na foto é lido e as cores são
//! convertidas para o espaço pedido (sRGB na tela; sRGB ou Adobe RGB na impressão). Foto sem
//! perfil é sRGB (ADR 0007) e, em sRGB, passa sem conversão nenhuma (regra 7).

use std::path::Path;

use image::{ImageDecoder, ImageReader, RgbaImage};
use lamina_core::ColorSpace;
use moxcms::{ColorProfile, Layout, RenderingIntent, TransformOptions};

/// O perfil do espaço de destino, como o `moxcms` o descreve.
fn target(space: ColorSpace) -> ColorProfile {
    match space {
        ColorSpace::Srgb => ColorProfile::new_srgb(),
        ColorSpace::AdobeRgb => ColorProfile::new_adobe_rgb(),
    }
}

/// O perfil ICC embutido no arquivo, se houver.
pub fn embedded_profile(path: &Path) -> Option<Vec<u8>> {
    let mut decoder = ImageReader::open(path)
        .ok()?
        .with_guessed_format()
        .ok()?
        .into_decoder()
        .ok()?;
    decoder.icc_profile().ok().flatten()
}

/// O aviso da importação quando a foto traz um perfil que não dá para ler (seção 6).
pub fn profile_problem(path: &Path) -> Option<String> {
    problem_of(path, &embedded_profile(path)?)
}

/// O mesmo aviso, a partir do perfil já lido com a foto (spec TR, regra 2: sem ler de novo).
pub fn problem_of(path: &Path, icc: &[u8]) -> Option<String> {
    ColorProfile::new_from_slice(icc).err().map(|_| {
        let name = path
            .file_name()
            .unwrap_or(path.as_os_str())
            .to_string_lossy();
        format!("A foto {name} tem um perfil de cor que não foi possível ler; usada como sRGB.")
    })
}

/// Converte os pixels do perfil `icc` (nenhum = sRGB) para `space`. Perfil ilegível conta como
/// sRGB. Sem nada a fazer (sRGB → sRGB, ou um perfil equivalente ao sRGB), devolve a imagem como
/// veio.
pub fn convert(image: RgbaImage, icc: Option<&[u8]>, space: ColorSpace) -> RgbaImage {
    let source = icc.and_then(|bytes| ColorProfile::new_from_slice(bytes).ok());
    let Some(source) = source.or_else(|| (space != ColorSpace::Srgb).then(ColorProfile::new_srgb))
    else {
        return image;
    };
    let destination = target(space);
    let options = TransformOptions {
        rendering_intent: RenderingIntent::Perceptual,
        ..TransformOptions::default()
    };
    // Em 16 bits: a conversão de 8 bits do `moxcms` usa uma curva de 4096 pontos e erra até 4
    // níveis nos tons escuros de uma ida e volta; a de 16 bits, no máximo 1.
    let Ok(transform) =
        source.create_transform_16bit(Layout::Rgba, &destination, Layout::Rgba, options)
    else {
        // Perfil de outro tipo (CMYK, cinza): fica como o decodificador entregou (seção 10).
        return image;
    };
    // Foto já no espaço da saída (o perfil dela equivale ao de destino): nada a converter.
    if icc.is_some() && is_identity(&*transform) {
        return image;
    }
    let (w, h) = image.dimensions();
    match apply(&*transform, image.as_raw()) {
        Some(out) => RgbaImage::from_raw(w, h, out).unwrap_or(image),
        None => image,
    }
}

/// Pixels por trecho: a conversão em 16 bits vai aos poucos, sem dobrar a memória da foto.
const CHUNK: usize = 16_384;

/// Converte RGBA de 8 bits passando por 16 bits, trecho a trecho.
fn apply(transform: &dyn moxcms::TransformExecutor<u16>, src: &[u8]) -> Option<Vec<u8>> {
    let mut out = Vec::with_capacity(src.len());
    let (mut wide, mut converted) = (Vec::new(), Vec::new());
    for chunk in src.chunks(CHUNK * 4) {
        wide.clear();
        wide.extend(chunk.iter().map(|&v| u16::from(v) * 257));
        converted.resize(wide.len(), 0);
        transform.transform(&wide, &mut converted).ok()?;
        out.extend(converted.iter().map(|&v| narrow(v)));
    }
    Some(out)
}

/// 16 bits → 8 bits, arredondando.
fn narrow(v: u16) -> u8 {
    u8::try_from((u32::from(v) + 128) / 257).unwrap_or(u8::MAX)
}

/// O perfil da foto é, na prática, o do destino? Confere uma grade de cores: se nenhuma muda, a
/// conversão é pulada (fotos de câmera costumam trazer o sRGB embutido).
fn is_identity(transform: &dyn moxcms::TransformExecutor<u16>) -> bool {
    const STEPS: [u8; 8] = [0, 36, 73, 109, 146, 182, 219, 255];
    let mut src = Vec::with_capacity(STEPS.len().pow(3) * 4);
    for r in STEPS {
        for g in STEPS {
            for b in STEPS {
                src.extend([r, g, b, 255]);
            }
        }
    }
    apply(transform, &src).is_some_and(|dst| dst == src)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn gradient() -> RgbaImage {
        RgbaImage::from_fn(64, 16, |x, y| {
            let v = |n: u32| u8::try_from(n.min(255)).unwrap();
            image::Rgba([v(x * 4), v(y * 16), v(255 - x * 4), 255])
        })
    }

    #[test]
    fn sem_perfil_em_srgb_fica_igual() {
        let image = gradient();
        assert_eq!(convert(image.clone(), None, ColorSpace::Srgb), image);
    }

    #[test]
    fn perfil_srgb_embutido_fica_igual() {
        let icc = ColorProfile::new_srgb().encode().unwrap();
        let image = gradient();
        assert_eq!(convert(image.clone(), Some(&icc), ColorSpace::Srgb), image);
    }

    #[test]
    fn perfil_ilegivel_conta_como_srgb() {
        let image = gradient();
        assert_eq!(
            convert(image.clone(), Some(b"lixo"), ColorSpace::Srgb),
            image
        );
    }
}
