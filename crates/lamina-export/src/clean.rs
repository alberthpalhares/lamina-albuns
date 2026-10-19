//! Limpeza da saída ao substituir (spec EI, regra 4): os arquivos com o nosso padrão de nome
//! (`lamina-NN`, `pagina-NN`, em `.jpg` ou `.tif`) do formato desta exportação que não fazem
//! parte dela saem da pasta. Nenhum outro arquivo é tocado.

use std::path::{Path, PathBuf};

use crate::ExportError;

/// O nome é de um arquivo que a exportação grava (`lamina-01.jpg`, `pagina-007.tif`)?
pub fn is_export_name(name: &str) -> bool {
    let stem = name
        .strip_suffix(".jpg")
        .or_else(|| name.strip_suffix(".tif"));
    let Some(number) = stem.and_then(|n| {
        n.strip_prefix("lamina-")
            .or_else(|| n.strip_prefix("pagina-"))
    }) else {
        return false;
    };
    number.len() >= 2 && number.bytes().all(|b| b.is_ascii_digit())
}

/// Apaga de `folder` (sem entrar nas subpastas) os arquivos com o nosso padrão de nome e a
/// extensão `ext` desta exportação que não estão em `keep`: substituir em TIFF não apaga os JPEGs
/// da mesma pasta, nem o contrário (auditoria do Marco 3, D1). Devolve os apagados, em ordem.
pub fn remove_leftovers(
    folder: &Path,
    keep: &[String],
    ext: &str,
) -> Result<Vec<PathBuf>, ExportError> {
    let fail = |path: &Path, e: std::io::Error| ExportError::Remove {
        path: path.display().to_string(),
        reason: e.to_string(),
    };
    let mut leftovers: Vec<PathBuf> = std::fs::read_dir(folder)
        .map_err(|e| fail(folder, e))?
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_ok_and(|t| t.is_file()))
        .filter_map(|entry| {
            let name = entry.file_name().to_str()?.to_owned();
            let ours = is_export_name(&name) && name.ends_with(ext);
            (ours && !keep.contains(&name)).then(|| entry.path())
        })
        .collect();
    leftovers.sort();
    for path in &leftovers {
        std::fs::remove_file(path).map_err(|e| fail(path, e))?;
    }
    Ok(leftovers)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn substituir_em_tiff_nao_apaga_os_jpegs() {
        let dir = tempfile::tempdir().unwrap();
        for name in [
            "lamina-01.jpg",
            "lamina-02.jpg",
            "lamina-01.tif",
            "lamina-09.tif",
        ] {
            std::fs::write(dir.path().join(name), b"x").unwrap();
        }
        let removed = remove_leftovers(dir.path(), &["lamina-01.tif".into()], ".tif").unwrap();
        assert_eq!(removed, [dir.path().join("lamina-09.tif")]);
        assert!(dir.path().join("lamina-02.jpg").exists());
    }

    #[test]
    fn so_o_nosso_padrao_de_nome() {
        assert!(is_export_name("lamina-01.jpg"));
        assert!(is_export_name("pagina-120.jpg"));
        assert!(is_export_name("lamina-03.tif"));
        for other in [
            "lamina-1.jpg",
            "lamina-01.png",
            "lamina-0a.jpg",
            "minha-lamina-01.jpg",
            "notas.txt",
            "lamina-.jpg",
        ] {
            assert!(!is_export_name(other), "{other}");
        }
    }
}
