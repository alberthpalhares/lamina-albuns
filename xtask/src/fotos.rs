//! `cargo xtask fotos <pasta> [quantas]`: fotos sintéticas para o teste de fumaça da release
//! (spec DS, regra 12), que monta um álbum pequeno com o `lamina build` em cada sistema.

use std::path::Path;

/// Grava `count` JPEGs 600 × 400 (uma vertical a cada quatro), diferentes entre si.
pub fn write(folder: &Path, count: u32) -> Result<(), String> {
    std::fs::create_dir_all(folder).map_err(|e| e.to_string())?;
    for i in 0..count {
        let (w, h) = if i % 4 == 3 { (400, 600) } else { (600, 400) };
        let tone = u8::try_from(i * 37 % 256).unwrap_or(0);
        let img = image::RgbImage::from_fn(w, h, |x, y| {
            let r = u8::try_from(x * 255 / w).unwrap_or(0);
            let g = u8::try_from(y * 255 / h).unwrap_or(0);
            image::Rgb([r, g, tone])
        });
        img.save(folder.join(format!("foto{i:02}.jpg")))
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

pub fn run(args: &[String]) -> bool {
    let (folder, count) = match args {
        [folder] => (folder, 8),
        [folder, n] => match n.parse() {
            Ok(n) => (folder, n),
            Err(_) => {
                eprintln!("uso: cargo xtask fotos <pasta> [quantas]");
                return false;
            }
        },
        _ => {
            eprintln!("uso: cargo xtask fotos <pasta> [quantas]");
            return false;
        }
    };
    match write(Path::new(folder), count) {
        Ok(()) => {
            println!("  fotos: {count} em {folder}");
            true
        }
        Err(e) => {
            eprintln!("  ✘ fotos: {e}");
            false
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn grava_as_fotos_pedidas() {
        let dir = tempfile::tempdir().unwrap();
        super::write(dir.path(), 5).unwrap();
        let count = std::fs::read_dir(dir.path()).unwrap().count();
        assert_eq!(count, 5);
        let vertical = image::image_dimensions(dir.path().join("foto03.jpg")).unwrap();
        assert_eq!(vertical, (400, 600));
    }
}
