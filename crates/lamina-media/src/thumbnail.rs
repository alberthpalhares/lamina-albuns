//! Miniaturas das fotos, já na orientação certa, guardadas como JPEG no cache.

use std::path::{Path, PathBuf};

use image::codecs::jpeg::JpegEncoder;
use image::{DynamicImage, ImageFormat};

use crate::{MediaError, decode_with_orientation};

/// Lado maior das miniaturas, em pixels: nítido na lista e na visão geral das lâminas.
pub const THUMB_SIZE: u32 = 320;

/// Decodifica a foto, aplica a rotação EXIF e reduz para caber em `max_side`.
pub fn make_thumbnail(
    path: &Path,
    exif_orientation: u8,
    max_side: u32,
) -> Result<DynamicImage, MediaError> {
    Ok(decode_with_orientation(path, exif_orientation)?.thumbnail(max_side, max_side))
}

/// Lado maior da imagem de tela: a lâmina grande na janela (spec JP, regra 6).
pub const SCREEN_SIZE: u32 = 1600;
/// Qualidade do JPEG da imagem de tela: só para mostrar, nunca para imprimir.
const SCREEN_QUALITY: u8 = 90;

/// Pastas do cache onde ficam as cópias reduzidas de cada foto.
#[derive(Debug, Clone, Copy)]
pub struct DerivedDirs<'a> {
    pub thumbs: &'a Path,
    /// Imagens de tela; `None` quando só a miniatura interessa (linha de comando).
    pub screens: Option<&'a Path>,
}

/// Versão das cópias reduzidas: a 2 converte as fotos com perfil próprio para sRGB (spec CT,
/// regra 10). As cópias de outra versão são ignoradas e refeitas.
const DERIVED_VERSION: u32 = 2;

/// Arquivo da cópia de uma foto (miniatura ou imagem de tela) dentro de `dir`.
pub fn derived_path(dir: &Path, fingerprint: &str) -> PathBuf {
    dir.join(format!("{fingerprint}.v{DERIVED_VERSION}.jpg"))
}

/// Gera (se ainda não existir) a miniatura em `dir/<impressão digital>.jpg` e devolve o caminho.
pub fn ensure_thumbnail(
    path: &Path,
    fingerprint: &str,
    exif_orientation: u8,
    dir: &Path,
) -> Result<PathBuf, MediaError> {
    let dirs = DerivedDirs {
        thumbs: dir,
        screens: None,
    };
    ensure_derived(path, fingerprint, exif_orientation, dirs)
}

/// Gera o que faltar entre a miniatura e a imagem de tela, decodificando a foto uma vez só, e
/// devolve o caminho da miniatura.
pub fn ensure_derived(
    path: &Path,
    fingerprint: &str,
    exif_orientation: u8,
    dirs: DerivedDirs<'_>,
) -> Result<PathBuf, MediaError> {
    let thumb = derived_path(dirs.thumbs, fingerprint);
    let screen = dirs
        .screens
        .map(|d| derived_path(d, fingerprint))
        .filter(|s| !s.exists());
    if thumb.exists() && screen.is_none() {
        return Ok(thumb);
    }
    let image = decode_with_orientation(path, exif_orientation)?;
    if !thumb.exists() {
        std::fs::create_dir_all(dirs.thumbs).map_err(MediaError::io(dirs.thumbs))?;
        image
            .thumbnail(THUMB_SIZE, THUMB_SIZE)
            .to_rgb8()
            .save_with_format(&thumb, ImageFormat::Jpeg)
            .map_err(MediaError::image(&thumb))?;
    }
    if let Some(screen) = screen {
        save_screen(&image, &screen)?;
    }
    Ok(thumb)
}

/// Grava a imagem de tela (até [`SCREEN_SIZE`] no lado maior; foto menor fica do tamanho dela).
fn save_screen(image: &DynamicImage, target: &Path) -> Result<(), MediaError> {
    if let Some(dir) = target.parent() {
        std::fs::create_dir_all(dir).map_err(MediaError::io(dir))?;
    }
    let reduced = if image.width().max(image.height()) > SCREEN_SIZE {
        image.thumbnail(SCREEN_SIZE, SCREEN_SIZE)
    } else {
        image.clone()
    };
    let file = std::fs::File::create(target).map_err(MediaError::io(target))?;
    let mut writer = std::io::BufWriter::new(file);
    let encoder = JpegEncoder::new_with_quality(&mut writer, SCREEN_QUALITY);
    reduced
        .to_rgb8()
        .write_with_encoder(encoder)
        .map_err(MediaError::image(target))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::write_jpeg;

    #[test]
    fn miniatura_cabe_no_tamanho_maximo_e_mantem_a_proporcao() {
        let dir = tempfile::tempdir().unwrap();
        let path = write_jpeg(dir.path(), "a.jpg", 900, 600, None);
        let thumb = make_thumbnail(&path, 1, 300).unwrap();
        assert_eq!((thumb.width(), thumb.height()), (300, 200));
    }

    #[test]
    fn rotacao_exif_e_aplicada() {
        let dir = tempfile::tempdir().unwrap();
        let path = write_jpeg(dir.path(), "a.jpg", 900, 600, None);
        let thumb = make_thumbnail(&path, 6, 300).unwrap();
        assert_eq!((thumb.width(), thumb.height()), (200, 300));
    }

    #[test]
    fn miniatura_existente_nao_e_refeita() {
        let dir = tempfile::tempdir().unwrap();
        let path = write_jpeg(dir.path(), "a.jpg", 90, 60, None);
        let cache = dir.path().join("cache");
        let first = ensure_thumbnail(&path, "abc", 1, &cache).unwrap();
        let modified = std::fs::metadata(&first).unwrap().modified().unwrap();
        let second = ensure_thumbnail(&path, "abc", 1, &cache).unwrap();
        assert_eq!(first, second);
        assert_eq!(
            std::fs::metadata(&second).unwrap().modified().unwrap(),
            modified
        );
    }

    #[test]
    fn imagem_de_tela_tem_1600_no_lado_maior_e_sai_com_a_miniatura() {
        let dir = tempfile::tempdir().unwrap();
        let path = write_jpeg(dir.path(), "a.jpg", 3000, 2000, None);
        let (thumbs, screens) = (dir.path().join("m"), dir.path().join("t"));
        let dirs = DerivedDirs {
            thumbs: &thumbs,
            screens: Some(&screens),
        };
        let thumb = ensure_derived(&path, "abc", 6, dirs).unwrap();
        let screen = image::open(derived_path(&screens, "abc")).unwrap();
        assert_eq!((screen.width(), screen.height()), (1067, 1600));
        assert_eq!(image::open(thumb).unwrap().height(), THUMB_SIZE);
    }

    #[test]
    fn foto_pequena_nao_e_ampliada_na_imagem_de_tela() {
        let dir = tempfile::tempdir().unwrap();
        let path = write_jpeg(dir.path(), "a.jpg", 900, 600, None);
        let screens = dir.path().join("t");
        let dirs = DerivedDirs {
            thumbs: dir.path(),
            screens: Some(&screens),
        };
        ensure_derived(&path, "abc", 1, dirs).unwrap();
        let screen = image::open(derived_path(&screens, "abc")).unwrap();
        assert_eq!((screen.width(), screen.height()), (900, 600));
    }
}
