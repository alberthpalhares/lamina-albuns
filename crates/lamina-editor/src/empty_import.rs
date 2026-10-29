//! Por que uma importação não trouxe nenhuma foto (relato do teste pelo MCP, 2026-10-29): a
//! pasta não tem foto, só tem outros arquivos ou tem fotos que não puderam ser lidas (sem
//! permissão, disco desconectado, arquivo estragado). Cada caso diz o que fazer.

use std::path::Path;

use lamina_media::ImportReport;

use crate::EditorError;

/// O erro de uma importação sem foto nenhuma: `place` é a pasta (ou o primeiro arquivo) e
/// `others`, quantos arquivos dela não são foto.
pub fn nothing_imported(place: &Path, report: &ImportReport, others: usize) -> EditorError {
    let folder = place.display().to_string();
    if let Some((path, reason)) = report.failures.first() {
        let file = path.file_name().map_or_else(
            || path.display().to_string(),
            |n| n.to_string_lossy().into_owned(),
        );
        return EditorError::NoPhotoReadable {
            folder,
            found: report.failures.len(),
            first: format!("{file}: {reason}"),
        };
    }
    if others > 0 {
        return EditorError::OnlyOtherFiles { folder, others };
    }
    EditorError::NoPhotos(folder)
}

/// Quantos arquivos da pasta (com as subpastas) não são foto; 0 se nem der para listar.
pub fn other_files(folder: &Path) -> usize {
    lamina_media::gather_photos(&[folder.to_path_buf()]).map_or(0, |g| g.not_photos)
}

/// "1 arquivo", "3 arquivos" e o mesmo para fotos.
pub fn files(n: usize, word: &str) -> String {
    if n == 1 {
        format!("1 {word}")
    } else {
        format!("{n} {word}s")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn report(failures: &[(&str, &str)]) -> ImportReport {
        ImportReport {
            failures: failures
                .iter()
                .map(|(p, r)| (Path::new(p).to_path_buf(), (*r).to_owned()))
                .collect(),
            ..ImportReport::default()
        }
    }

    #[test]
    fn fotos_que_nao_puderam_ser_lidas_dizem_quantas_e_o_primeiro_motivo() {
        let failed = report(&[
            ("/fotos/a.jpg", "sem permissão de acesso"),
            ("/fotos/b.jpg", "sem permissão de acesso"),
        ]);
        let err = nothing_imported(Path::new("/fotos"), &failed, 0);
        assert!(matches!(err, EditorError::NoPhotoReadable { found: 2, .. }));
        assert_eq!(
            err.to_string(),
            "Nenhuma das 2 fotos de /fotos pôde ser lida. Primeiro motivo: a.jpg: sem permissão \
             de acesso."
        );
    }

    #[test]
    fn pasta_so_com_outros_arquivos_diz_quantos() {
        let err = nothing_imported(Path::new("/docs"), &report(&[]), 1);
        assert!(matches!(err, EditorError::OnlyOtherFiles { others: 1, .. }));
        assert_eq!(
            err.to_string(),
            "Nenhuma foto encontrada em /docs: 1 arquivo, mas nenhum em JPEG, PNG, TIFF ou HEIC."
        );
    }

    #[test]
    fn pasta_vazia_continua_sem_fotos() {
        let err = nothing_imported(Path::new("/vazia"), &report(&[]), 0);
        assert!(matches!(err, EditorError::NoPhotos(_)));
    }
}
