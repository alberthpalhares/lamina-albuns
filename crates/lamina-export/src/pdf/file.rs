//! O arquivo PDF gravado aos poucos (spec PD, regra 9): cada objeto é escrito no disco assim que
//! fica pronto, e o arquivo guarda só onde cada um começou. No fim vêm a tabela de referências
//! (xref) e o trailer. Assim a memória não cresce com o número de lâminas.
//!
//! Cada bloco (`Chunk`) do `pdf-writer` passado aqui tem **um** objeto só, que começa no início
//! do bloco: é assim que se sabe onde ele fica no arquivo.

use std::io::{BufWriter, Write as _};
use std::path::{Path, PathBuf};

use pdf_writer::{Chunk, Ref};
use tempfile::NamedTempFile;

use crate::ExportError;

/// Cabeçalho do PDF 1.6 (a base do PDF/X-4), com a linha de bytes altos que diz "binário".
const HEADER: &[u8] = b"%PDF-1.6\n%\xE2\xE3\xCF\xD3\n";

pub struct PdfFile {
    out: BufWriter<NamedTempFile>,
    path: PathBuf,
    written: usize,
    offsets: Vec<(i32, usize)>,
    hash: Fnv,
}

impl PdfFile {
    /// Começa o arquivo num temporário da mesma pasta (some sozinho se algo falhar).
    pub fn create(folder: &Path, path: &Path) -> Result<Self, ExportError> {
        let fail = write_error(path);
        let tmp = tempfile::Builder::new()
            .prefix(".lamina-")
            .suffix(".tmp")
            .tempfile_in(folder)
            .map_err(&fail)?;
        let mut file = PdfFile {
            out: BufWriter::new(tmp),
            path: path.to_path_buf(),
            written: 0,
            offsets: Vec::new(),
            hash: Fnv::new(),
        };
        file.raw(HEADER)?;
        Ok(file)
    }

    /// Grava o objeto `id`, o único do bloco.
    pub fn object(&mut self, id: Ref, chunk: &Chunk) -> Result<(), ExportError> {
        self.offsets.push((id.get(), self.written));
        self.hash.update(chunk.as_bytes());
        self.raw(chunk.as_bytes())
    }

    /// Identificador do arquivo, tirado de tudo o que foi gravado até aqui (o mesmo projeto dá o
    /// mesmo identificador: regra 12).
    pub fn digest(&self) -> [u8; 16] {
        self.hash.finish()
    }

    /// Tabela de referências e trailer; renomeia o temporário para o nome final.
    pub fn finish(mut self, root: Ref, info: Ref) -> Result<PathBuf, ExportError> {
        self.offsets.sort_unstable();
        let size = self.offsets.len() + 1;
        let contiguous = self
            .offsets
            .iter()
            .enumerate()
            .all(|(i, (id, _))| usize::try_from(*id).is_ok_and(|id| id == i + 1));
        if !contiguous {
            return Err(write_error(&self.path)(std::io::Error::other(
                "objetos do PDF fora de ordem",
            )));
        }
        let xref_at = self.written;
        let mut tail = format!("xref\n0 {size}\n0000000000 65535 f \n");
        for (_, offset) in &self.offsets {
            tail.push_str(&format!("{offset:010} 00000 n \n"));
        }
        let id: String = self.digest().iter().map(|b| format!("{b:02X}")).collect();
        tail.push_str(&format!(
            "trailer\n<<\n  /Size {size}\n  /Root {} 0 R\n  /Info {} 0 R\n  /ID [<{id}> <{id}>]\n>>\nstartxref\n{xref_at}\n%%EOF\n",
            root.get(),
            info.get()
        ));
        self.raw(tail.as_bytes())?;
        let path = self.path;
        {
            let fail = write_error(&path);
            let tmp = self.out.into_inner().map_err(|e| fail(e.into_error()))?;
            tmp.as_file().sync_all().map_err(&fail)?;
            tmp.persist(&path).map_err(|e| fail(e.error))?;
        }
        Ok(path)
    }

    fn raw(&mut self, bytes: &[u8]) -> Result<(), ExportError> {
        self.out.write_all(bytes).map_err(write_error(&self.path))?;
        self.written += bytes.len();
        Ok(())
    }
}

fn write_error(path: &Path) -> impl Fn(std::io::Error) -> ExportError + '_ {
    move |e| ExportError::Write {
        path: path.display().to_string(),
        reason: e.to_string(),
    }
}

/// FNV-1a de 128 bits (dois de 64 com sementes diferentes): sempre o mesmo resultado, em
/// qualquer máquina e versão do Rust.
struct Fnv([u64; 2]);

impl Fnv {
    const PRIME: u64 = 0x0000_0100_0000_01B3;

    fn new() -> Self {
        Fnv([0xcbf2_9ce4_8422_2325, 0x84222325_cbf29ce4])
    }

    fn update(&mut self, bytes: &[u8]) {
        for &b in bytes {
            for h in &mut self.0 {
                *h ^= u64::from(b);
                *h = h.wrapping_mul(Self::PRIME);
            }
        }
    }

    fn finish(&self) -> [u8; 16] {
        let mut out = [0; 16];
        out[..8].copy_from_slice(&self.0[0].to_be_bytes());
        out[8..].copy_from_slice(&self.0[1].to_be_bytes());
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identificador_depende_so_do_conteudo() {
        let (mut a, mut b, mut c) = (Fnv::new(), Fnv::new(), Fnv::new());
        a.update(b"lamina 1");
        b.update(b"lamina 1");
        c.update(b"lamina 2");
        assert_eq!(a.finish(), b.finish());
        assert_ne!(a.finish(), c.finish());
    }
}
