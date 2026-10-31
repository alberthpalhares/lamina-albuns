//! TIFF de impressão sem perda (spec CT, regra 5): RGB de 8 bits, compressão Deflate, perfil de
//! cor no espaço da saída e resolução em pontos por polegada.

use std::borrow::Cow;
use std::io::Cursor;

use image::RgbaImage;
use tiff::encoder::colortype::RGB8;
use tiff::encoder::{Compression, DeflateLevel, Rational, TiffEncoder, TiffValue};
use tiff::tags::{ResolutionUnit, Tag, Type};

/// O perfil ICC vai no tipo UNDEFINED, como pede a nota técnica do ICC sobre TIFF.
struct Undefined<'a>(&'a [u8]);

impl TiffValue for Undefined<'_> {
    const BYTE_LEN: u8 = 1;
    const FIELD_TYPE: Type = Type::UNDEFINED;

    fn count(&self) -> usize {
        self.0.len()
    }

    fn data(&self) -> Cow<'_, [u8]> {
        Cow::Borrowed(self.0)
    }
}

/// Codifica a imagem em TIFF com o perfil `profile` e densidade `dpi`.
pub fn encode(image: RgbaImage, dpi: u32, profile: &[u8]) -> Result<Vec<u8>, String> {
    let (width, height) = image.dimensions();
    let rgb = crate::jpeg::drop_alpha(image);
    let mut out = Cursor::new(Vec::new());
    let mut encoder = TiffEncoder::new(&mut out)
        .map_err(|e| e.to_string())?
        .with_compression(Compression::Deflate(DeflateLevel::Balanced));
    let mut tiff = encoder
        .new_image::<RGB8>(width, height)
        .map_err(|e| e.to_string())?;
    tiff.encoder()
        .write_tag(Tag::IccProfile, Undefined(profile))
        .map_err(|e| e.to_string())?;
    tiff.resolution(ResolutionUnit::Inch, Rational { n: dpi, d: 1 });
    tiff.write_data(&rgb).map_err(|e| e.to_string())?;
    Ok(out.into_inner())
}
