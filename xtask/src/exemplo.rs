//! O álbum de exemplo (spec PC, regras 5 e 6). `cargo xtask exemplo <pasta>` prepara as fotos
//! escolhidas: viradas e em sRGB, lado maior de até 1600 pixels, JPEG sem os dados da câmera e
//! nomes simples. O teste `pc_b5` confere o exemplo do repositório (a trava da spec).

use std::fs;
use std::path::{Path, PathBuf};

use image::imageops::FilterType;
use image::{DynamicImage, codecs::jpeg::JpegEncoder};
use lamina_editor::{EXAMPLE_FILE, Editor};

/// A pasta do exemplo no repositório.
pub const EXAMPLE: &str = "crates/lamina-app/exemplo";
/// Até 8 MB no total, para o instalador não crescer demais.
pub const MAX_BYTES: u64 = 8 * 1024 * 1024;
pub const LONG_SIDE: u32 = 1600;
const QUALITY: u8 = 78;
const PHOTOS: std::ops::RangeInclusive<usize> = 16..=20;
const SPREADS: std::ops::RangeInclusive<usize> = 6..=8;

fn photos_in(dir: &Path) -> Vec<PathBuf> {
    let mut list: Vec<PathBuf> = fs::read_dir(dir)
        .map(|entries| entries.flatten().map(|e| e.path()).collect())
        .unwrap_or_default();
    list.retain(|p| {
        p.extension()
            .and_then(|e| e.to_str())
            .is_some_and(|e| matches!(e.to_lowercase().as_str(), "jpg" | "jpeg" | "png"))
    });
    list.sort();
    list
}

/// Uma foto preparada: virada, em sRGB, reduzida e regravada sem metadados.
fn prepare(from: &Path, to: &Path) -> Result<(), String> {
    let rgba = lamina_media::load_oriented(from).map_err(|e| e.to_string())?;
    let mut image = DynamicImage::ImageRgba8(rgba);
    if image.width().max(image.height()) > LONG_SIDE {
        image = image.resize(LONG_SIDE, LONG_SIDE, FilterType::Lanczos3);
    }
    let file = fs::File::create(to).map_err(|e| format!("{}: {e}", to.display()))?;
    let mut out = std::io::BufWriter::new(file);
    JpegEncoder::new_with_quality(&mut out, QUALITY)
        .encode_image(&DynamicImage::ImageRgb8(image.to_rgb8()))
        .map_err(|e| format!("{}: {e}", to.display()))
}

/// `cargo xtask exemplo <pasta>` prepara as fotos e monta o álbum; `--album` só monta o álbum.
pub fn run(args: &[String]) -> bool {
    let ok = match args {
        [flag] if flag == "--album" => true,
        [from] => photos(from),
        _ => {
            eprintln!("uso: cargo xtask exemplo <pasta com as fotos escolhidas> | --album");
            return false;
        }
    };
    if !ok {
        return false;
    }
    if let Err(e) = crate::exemplo_album::build(Path::new(EXAMPLE)) {
        eprintln!("  ✘ exemplo: o álbum não foi montado: {e}");
        return false;
    }
    let found = problems(Path::new(EXAMPLE));
    for p in &found {
        eprintln!("  ✘ {p}");
    }
    if found.is_empty() {
        println!("  exemplo ok");
    }
    found.is_empty()
}

fn photos(from: &str) -> bool {
    let to = Path::new(EXAMPLE).join("fotos");
    if let Err(e) = fs::create_dir_all(&to) {
        eprintln!("  ✘ exemplo: {e}");
        return false;
    }
    for (i, photo) in photos_in(Path::new(from)).iter().enumerate() {
        let name = format!("exemplo-{:02}.jpg", i + 1);
        if let Err(e) = prepare(photo, &to.join(&name)) {
            eprintln!("  ✘ exemplo: {}: {e}", photo.display());
            return false;
        }
        println!("  {name} ← {}", photo.display());
    }
    true
}

/// A foto tem dados da câmera (EXIF) ou do editor (XMP)?
pub fn has_camera_data(bytes: &[u8]) -> bool {
    let head = &bytes[..bytes.len().min(256 * 1024)];
    let has = |needle: &[u8]| head.windows(needle.len()).any(|w| w == needle);
    has(b"Exif\0\0") || has(b"http://ns.adobe.com/xap/")
}

/// O que impede o exemplo de estar pronto para leigos (PC-B5).
pub fn problems(dir: &Path) -> Vec<String> {
    let mut found = Vec::new();
    let photos = photos_in(&dir.join("fotos"));
    if !PHOTOS.contains(&photos.len()) {
        found.push(format!("{} fotos; o exemplo pede de 16 a 20", photos.len()));
    }
    let credits = fs::read_to_string(dir.join("CREDITOS.md")).unwrap_or_default();
    let mut total = 0;
    for photo in &photos {
        let name = photo
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        let bytes = fs::read(photo).unwrap_or_default();
        total += bytes.len() as u64;
        if has_camera_data(&bytes) {
            found.push(format!("{name} tem dados da câmera (EXIF ou XMP)"));
        }
        if !credits
            .lines()
            .any(|l| l.contains(&name) && l.contains("CC0 1.0"))
        {
            found.push(format!("{name} sem crédito CC0 1.0 no CREDITOS.md"));
        }
    }
    if total > MAX_BYTES {
        found.push(format!("{total} bytes; o limite é {MAX_BYTES}"));
    }
    found.extend(album_problems(&dir.join(EXAMPLE_FILE)));
    found
}

fn album_problems(file: &Path) -> Vec<String> {
    let mut editor = Editor::new();
    if let Err(e) = editor.open(file) {
        return vec![format!("{} não abre: {e}", file.display())];
    }
    let Some(session) = editor.session() else {
        return vec!["nenhum álbum aberto".to_owned()];
    };
    let mut found = Vec::new();
    if !session.missing().is_empty() {
        found.push(format!("{} foto(s) faltando", session.missing().len()));
    }
    if !SPREADS.contains(&session.spread_count()) {
        found.push(format!(
            "{} lâminas; o exemplo pede de 6 a 8",
            session.spread_count()
        ));
    }
    for i in 0..session.spread_count() {
        for warning in session.warnings_for(i) {
            found.push(format!("aviso de pré-impressão: {warning}"));
        }
    }
    found
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pc_b5_o_exemplo_do_repositorio_esta_pronto_para_leigos() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
        let found = problems(&root.join(EXAMPLE));
        assert!(found.is_empty(), "{found:#?}");
    }

    #[test]
    fn foto_com_dados_da_camera_e_apontada() {
        let mut jpeg = vec![0xFF, 0xD8, 0xFF, 0xE1, 0x00, 0x10];
        jpeg.extend_from_slice(b"Exif\0\0MM");
        assert!(has_camera_data(&jpeg));
        assert!(!has_camera_data(&[0xFF, 0xD8, 0xFF, 0xDB, 0x00, 0x43]));
    }
}
