//! Formatos de imagem reconhecidos e listagem das fotos de uma pasta.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// Formatos de imagem que o Lâmina importa.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageFormat {
    Jpeg,
    Png,
    Tiff,
    /// HEIC/HEIF (iPhone). Decodificação prevista para depois do MVP.
    Heic,
}

impl ImageFormat {
    /// Reconhece o formato pela extensão do arquivo, sem diferenciar maiúsculas.
    pub fn from_path(path: &Path) -> Option<Self> {
        let ext = path.extension()?.to_str()?.to_ascii_lowercase();
        match ext.as_str() {
            "jpg" | "jpeg" => Some(ImageFormat::Jpeg),
            "png" => Some(ImageFormat::Png),
            "tif" | "tiff" => Some(ImageFormat::Tiff),
            "heic" | "heif" => Some(ImageFormat::Heic),
            _ => None,
        }
    }

    /// `true` se o Lâmina já sabe decodificar este formato.
    pub fn is_decodable(self) -> bool {
        !matches!(self, ImageFormat::Heic)
    }
}

/// Lista as fotos de uma pasta e das subpastas, em ordem alfabética de caminho.
pub fn list_photos(dir: &Path) -> io::Result<Vec<PathBuf>> {
    let mut photos = Vec::new();
    let mut pending = vec![dir.to_path_buf()];
    while let Some(current) = pending.pop() {
        for entry in fs::read_dir(&current)? {
            let path = entry?.path();
            if path.is_dir() {
                pending.push(path);
            } else if ImageFormat::from_path(&path).is_some() {
                photos.push(path);
            }
        }
    }
    photos.sort();
    Ok(photos)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reconhece_extensoes_sem_diferenciar_maiusculas() {
        assert_eq!(
            ImageFormat::from_path(Path::new("a/IMG_1.JPG")),
            Some(ImageFormat::Jpeg)
        );
        assert_eq!(
            ImageFormat::from_path(Path::new("b.tiff")),
            Some(ImageFormat::Tiff)
        );
        assert_eq!(
            ImageFormat::from_path(Path::new("c.HEIC")),
            Some(ImageFormat::Heic)
        );
        assert_eq!(ImageFormat::from_path(Path::new("notas.txt")), None);
        assert_eq!(ImageFormat::from_path(Path::new("sem_extensao")), None);
    }

    #[test]
    fn lista_fotos_recursivamente_e_ignora_outros_arquivos() {
        let root = std::env::temp_dir().join(format!("lamina-media-teste-{}", std::process::id()));
        let sub = root.join("cerimonia");
        fs::create_dir_all(&sub).unwrap();
        fs::write(root.join("b.jpg"), b"").unwrap();
        fs::write(root.join("leiame.txt"), b"").unwrap();
        fs::write(sub.join("a.png"), b"").unwrap();

        let photos = list_photos(&root).unwrap();
        fs::remove_dir_all(&root).unwrap();

        assert_eq!(photos, vec![root.join("b.jpg"), sub.join("a.png")]);
    }
}
