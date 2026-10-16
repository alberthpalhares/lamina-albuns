//! Limpeza da saída ao substituir (spec EI, regra 4): os arquivos com o nosso padrão de nome
//! (`lamina-NN.jpg`, `pagina-NN.jpg`) que não fazem parte desta exportação saem da pasta.
//! Nenhum outro arquivo é tocado.

use std::path::{Path, PathBuf};

use crate::ExportError;

/// O nome é de um arquivo que a exportação grava (`lamina-01.jpg`, `pagina-007.jpg`)?
pub fn is_export_name(name: &str) -> bool {
    let Some(number) = name.strip_suffix(".jpg").and_then(|n| {
        n.strip_prefix("lamina-")
            .or_else(|| n.strip_prefix("pagina-"))
    }) else {
        return false;
    };
    number.len() >= 2 && number.bytes().all(|b| b.is_ascii_digit())
}

/// Apaga de `folder` (sem entrar nas subpastas) os arquivos com o nosso padrão de nome que não
/// estão em `keep`. Devolve os apagados, em ordem de nome.
pub fn remove_leftovers(folder: &Path, keep: &[String]) -> Result<Vec<PathBuf>, ExportError> {
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
            (is_export_name(&name) && !keep.contains(&name)).then(|| entry.path())
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
    fn so_o_nosso_padrao_de_nome() {
        assert!(is_export_name("lamina-01.jpg"));
        assert!(is_export_name("pagina-120.jpg"));
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
