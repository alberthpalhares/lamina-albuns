//! Impressão digital do conteúdo de uma foto (ADR 0004).
//!
//! Ler o arquivo inteiro seria lento para milhares de fotos de 20 MB. Usamos o tamanho mais os
//! primeiros e os últimos 64 KiB: o começo de um JPEG tem o EXIF (com data e número de série) e
//! o fim tem os últimos blocos da imagem. Duas fotos diferentes com esses trechos iguais e o
//! mesmo tamanho exato são, na prática, impossíveis.

use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;

use crate::MediaError;

/// Quantos bytes são lidos do começo e do fim do arquivo.
const SAMPLE: u64 = 64 * 1024;

/// Impressão digital em hexadecimal (32 caracteres).
pub fn fingerprint(path: &Path) -> Result<String, MediaError> {
    let mut file = File::open(path).map_err(MediaError::io(path))?;
    let len = file.metadata().map_err(MediaError::io(path))?.len();
    let mut hasher = blake3::Hasher::new();
    hasher.update(&len.to_le_bytes());

    let mut buffer = Vec::new();
    (&mut file)
        .take(SAMPLE)
        .read_to_end(&mut buffer)
        .map_err(MediaError::io(path))?;
    hasher.update(&buffer);

    if len > SAMPLE {
        let tail_start = (len - SAMPLE).max(SAMPLE);
        file.seek(SeekFrom::Start(tail_start))
            .map_err(MediaError::io(path))?;
        buffer.clear();
        file.read_to_end(&mut buffer)
            .map_err(MediaError::io(path))?;
        hasher.update(&buffer);
    }
    Ok(hasher.finalize().to_hex()[..32].to_owned())
}

/// A mesma impressão digital de [`fingerprint`], a partir do arquivo já lido inteiro (spec TR,
/// regra 2): a importação lê cada foto uma vez só.
pub fn fingerprint_of(bytes: &[u8]) -> String {
    let len = u64::try_from(bytes.len()).unwrap_or(u64::MAX);
    let mut hasher = blake3::Hasher::new();
    hasher.update(&len.to_le_bytes());
    let head = usize::try_from(SAMPLE)
        .unwrap_or(usize::MAX)
        .min(bytes.len());
    hasher.update(&bytes[..head]);
    if len > SAMPLE {
        let tail_start = (len - SAMPLE).max(SAMPLE);
        let tail_start = usize::try_from(tail_start).unwrap_or(bytes.len());
        hasher.update(&bytes[tail_start..]);
    }
    hasher.finalize().to_hex()[..32].to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write(dir: &Path, name: &str, bytes: &[u8]) -> std::path::PathBuf {
        let path = dir.join(name);
        std::fs::write(&path, bytes).unwrap();
        path
    }

    fn content(len: usize) -> Vec<u8> {
        (0..len).map(|i| u8::try_from(i % 251).unwrap()).collect()
    }

    #[test]
    fn mesmo_conteudo_mesma_impressao_mesmo_com_outro_nome() {
        let dir = tempfile::tempdir().unwrap();
        let data = content(300_000);
        let a = fingerprint(&write(dir.path(), "a.jpg", &data)).unwrap();
        let b = fingerprint(&write(dir.path(), "copia renomeada.jpg", &data)).unwrap();
        assert_eq!(a, b);
        assert_eq!(a.len(), 32);
    }

    #[test]
    fn mudanca_no_comeco_ou_no_fim_muda_a_impressao() {
        let dir = tempfile::tempdir().unwrap();
        let data = content(300_000);
        let original = fingerprint(&write(dir.path(), "a.jpg", &data)).unwrap();
        let mut head = data.clone();
        head[10] ^= 1;
        let mut tail = data.clone();
        tail[299_990] ^= 1;
        assert_ne!(
            original,
            fingerprint(&write(dir.path(), "b.jpg", &head)).unwrap()
        );
        assert_ne!(
            original,
            fingerprint(&write(dir.path(), "c.jpg", &tail)).unwrap()
        );
    }

    #[test]
    fn tr_b4_a_impressao_digital_dos_bytes_e_a_de_sempre() {
        let dir = tempfile::tempdir().unwrap();
        for len in [3, 64 * 1024, 64 * 1024 + 1, 100_000, 300_000] {
            let data = content(len);
            let path = write(dir.path(), &format!("{len}.jpg"), &data);
            assert_eq!(
                fingerprint(&path).unwrap(),
                fingerprint_of(&data),
                "{len} bytes"
            );
        }
    }

    #[test]
    fn arquivo_pequeno_e_lido_inteiro() {
        let dir = tempfile::tempdir().unwrap();
        let a = fingerprint(&write(dir.path(), "a.jpg", b"abc")).unwrap();
        let b = fingerprint(&write(dir.path(), "b.jpg", b"abd")).unwrap();
        assert_ne!(a, b);
    }
}
