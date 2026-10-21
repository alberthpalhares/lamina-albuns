//! `cargo xtask fotos <pasta> [quantas] [--lado N]`: fotos sintéticas para os testes de fumaça.
//! Sem `--lado`, JPEGs 600 × 400 para o álbum pequeno da release (spec DS, regra 12). Com
//! `--lado N`, fotos grandes (N no lado maior) para a importação de 1.000 fotos no CI (spec TR,
//! regra 9): 8 fotos de verdade e cópias delas com um comentário próprio, que muda a impressão
//! digital sem gastar tempo codificando cada uma.

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

/// Quantas fotos grandes são codificadas de verdade; as outras são cópias delas.
const LARGE_BASES: u32 = 8;

/// Grava `count` JPEGs com `side` pixels no lado maior (3:2, uma vertical a cada quatro).
pub fn write_large(folder: &Path, count: u32, side: u32) -> Result<(), String> {
    std::fs::create_dir_all(folder).map_err(|e| e.to_string())?;
    let mut bases = Vec::new();
    for i in 0..LARGE_BASES.min(count) {
        let (w, h) = if i % 4 == 3 {
            (side * 2 / 3, side)
        } else {
            (side, side * 2 / 3)
        };
        let img = image::RgbImage::from_fn(w, h, |x, y| {
            let v = |n: u32| u8::try_from(n % 256).unwrap_or(0);
            image::Rgb([v(x / 7 + i * 31), v(y / 5), v((x + y) / 9)])
        });
        let mut bytes = Vec::new();
        img.write_to(
            &mut std::io::Cursor::new(&mut bytes),
            image::ImageFormat::Jpeg,
        )
        .map_err(|e| e.to_string())?;
        bases.push(bytes);
    }
    for n in 0..count {
        let base = &bases[usize::try_from(n % LARGE_BASES).unwrap_or(0) % bases.len()];
        let bytes = with_comment(base, &format!("foto {n}"));
        std::fs::write(folder.join(format!("grande{n:04}.jpg")), bytes)
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// O JPEG `bytes` com um segmento de comentário logo depois do início: dentro dos primeiros
/// 64 KiB, então a impressão digital muda; os pixels, não.
fn with_comment(bytes: &[u8], text: &str) -> Vec<u8> {
    let len = u16::try_from(text.len() + 2).unwrap_or(u16::MAX);
    let mut out = bytes[..2].to_vec();
    out.extend([0xFF, 0xFE]);
    out.extend(len.to_be_bytes());
    out.extend(text.as_bytes());
    out.extend(&bytes[2..]);
    out
}

const USAGE: &str = "uso: cargo xtask fotos <pasta> [quantas] [--lado N]";

pub fn run(args: &[String]) -> bool {
    let (mut side, mut rest) = (None, Vec::new());
    let mut it = args.iter();
    while let Some(arg) = it.next() {
        if arg == "--lado" {
            // Lado ilegível vira 0, recusado abaixo.
            side = Some(it.next().and_then(|n| n.parse::<u32>().ok()).unwrap_or(0));
        } else {
            rest.push(arg);
        }
    }
    let parsed = match rest.as_slice() {
        [folder] => Some((*folder, 8)),
        [folder, n] => n.parse().ok().map(|n| (*folder, n)),
        _ => None,
    };
    let (Some((folder, count)), false) = (parsed, side == Some(0)) else {
        eprintln!("{USAGE}");
        return false;
    };
    let result = match side {
        Some(side) => write_large(Path::new(folder), count, side),
        None => write(Path::new(folder), count),
    };
    match result {
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

    #[test]
    fn fotos_grandes_sao_copias_com_impressao_digital_propria() {
        let dir = tempfile::tempdir().unwrap();
        assert!(super::run(&[
            dir.path().to_str().unwrap().to_owned(),
            "10".to_owned(),
            "--lado".to_owned(),
            "300".to_owned(),
        ]));
        let path = |n: u32| dir.path().join(format!("grande{n:04}.jpg"));
        assert_eq!(image::image_dimensions(path(0)).unwrap(), (300, 200));
        assert_eq!(image::image_dimensions(path(3)).unwrap(), (200, 300));
        // A foto 8 é cópia da 0: os mesmos pixels, outra impressão digital.
        let (a, b) = (image::open(path(0)).unwrap(), image::open(path(8)).unwrap());
        assert_eq!(a.as_bytes(), b.as_bytes());
        let fingerprint = lamina_media::fingerprint::fingerprint;
        assert_ne!(
            fingerprint(&path(0)).unwrap(),
            fingerprint(&path(8)).unwrap()
        );
    }
}
