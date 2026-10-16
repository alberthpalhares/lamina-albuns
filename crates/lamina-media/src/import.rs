//! Importação de uma pasta: metadados, impressão digital e miniatura de cada foto, em paralelo,
//! reaproveitando o que o catálogo já conhece.

use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::UNIX_EPOCH;

use rayon::prelude::*;

use crate::fingerprint::fingerprint;
use crate::thumbnail::{DerivedDirs, derived_path, ensure_derived};
use crate::{Catalog, CatalogEntry, ImageFormat, MediaError, list_photos, read_metadata};

/// Resultado de uma importação.
#[derive(Debug, Default)]
pub struct ImportReport {
    /// Todas as fotos importadas, em ordem de captura.
    pub entries: Vec<CatalogEntry>,
    /// Quantas vieram do catálogo sem serem lidas de novo.
    pub reused: usize,
    /// Quantas foram lidas agora.
    pub processed: usize,
    /// Arquivos que não puderam ser importados, com o motivo.
    pub failures: Vec<(PathBuf, String)>,
}

/// Andamento da importação: chamado com (fotos prontas, total), sempre em ordem crescente.
pub type Progress<'a> = &'a (dyn Fn(usize, usize) + Sync);

/// Importa as fotos de `dir` (e subpastas), gravando miniaturas em `thumbs_dir`.
pub fn import_folder(
    dir: &Path,
    catalog: &Catalog,
    thumbs_dir: &Path,
) -> Result<ImportReport, MediaError> {
    let dirs = DerivedDirs {
        thumbs: thumbs_dir,
        screens: None,
    };
    import_folder_with(dir, catalog, dirs, &|_, _| {})
}

/// Como [`import_folder`], gerando também as imagens de tela (se `dirs.screens` vier) e
/// informando o andamento.
pub fn import_folder_with(
    dir: &Path,
    catalog: &Catalog,
    dirs: DerivedDirs<'_>,
    progress: Progress<'_>,
) -> Result<ImportReport, MediaError> {
    let mut report = ImportReport::default();
    let mut pending = Vec::new();
    let photos = list_photos(dir).map_err(MediaError::io(dir))?;
    let total = photos.len();
    for path in photos {
        if !ImageFormat::from_path(&path).is_some_and(ImageFormat::is_decodable) {
            report
                .failures
                .push((path, "formato ainda não suportado".into()));
            continue;
        }
        let (size, modified) = file_stamp(&path)?;
        match catalog.get(&path)? {
            Some(e) if is_current(&e, size, modified, dirs) => {
                report.reused += 1;
                report.entries.push(e);
            }
            _ => pending.push((path, size, modified)),
        }
    }

    // Um cadeado em volta do contador e da chamada mantém as contagens em ordem crescente.
    let done = Mutex::new(total - pending.len());
    let step = || {
        if let Ok(mut n) = done.lock() {
            *n += 1;
            progress(*n, total);
        }
    };
    progress(total - pending.len(), total);
    let results: Vec<_> = pending
        .into_par_iter()
        .map(|(path, size, modified)| {
            let result = process(&path, size, modified, dirs).map_err(|e| (path, e.to_string()));
            step();
            result
        })
        .collect();
    for result in results {
        match result {
            Ok(entry) => {
                catalog.upsert(&entry)?;
                report.processed += 1;
                report.entries.push(entry);
            }
            Err(failure) => report.failures.push(failure),
        }
    }
    report.entries.sort_by(|a, b| {
        (a.taken_at.is_none(), a.taken_at, &a.path).cmp(&(
            b.taken_at.is_none(),
            b.taken_at,
            &b.path,
        ))
    });
    Ok(report)
}

/// A entrada do catálogo vale se o arquivo não mudou e as cópias reduzidas pedidas existem.
fn is_current(e: &CatalogEntry, size: i64, modified: i64, dirs: DerivedDirs<'_>) -> bool {
    let screen_ok = dirs
        .screens
        .is_none_or(|d| derived_path(d, &e.fingerprint).exists());
    e.file_size == size && e.modified == modified && e.thumbnail.exists() && screen_ok
}

/// Tamanho e data de modificação (segundos desde 1970) do arquivo.
fn file_stamp(path: &Path) -> Result<(i64, i64), MediaError> {
    let meta = std::fs::metadata(path).map_err(MediaError::io(path))?;
    let size = i64::try_from(meta.len()).unwrap_or(i64::MAX);
    let modified = meta
        .modified()
        .ok()
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .and_then(|d| i64::try_from(d.as_secs()).ok())
        .unwrap_or(0);
    Ok((size, modified))
}

fn process(
    path: &Path,
    size: i64,
    modified: i64,
    dirs: DerivedDirs<'_>,
) -> Result<CatalogEntry, MediaError> {
    let meta = read_metadata(path)?;
    let fingerprint = fingerprint(path)?;
    let thumbnail = ensure_derived(path, &fingerprint, meta.exif_orientation, dirs)?;
    Ok(CatalogEntry {
        path: path.to_path_buf(),
        fingerprint,
        file_size: size,
        modified,
        width: meta.width,
        height: meta.height,
        exif_orientation: meta.exif_orientation,
        taken_at: meta.taken_at,
        camera: meta.camera,
        thumbnail,
    })
}

/// Pasta de cache padrão do sistema para o Lâmina (catálogo e miniaturas).
pub fn default_cache_dir() -> PathBuf {
    let env = |name| std::env::var_os(name).map(PathBuf::from);
    let base = if cfg!(target_os = "windows") {
        env("LOCALAPPDATA")
    } else if cfg!(target_os = "macos") {
        env("HOME").map(|h| h.join("Library").join("Caches"))
    } else {
        env("XDG_CACHE_HOME").or_else(|| env("HOME").map(|h| h.join(".cache")))
    };
    base.unwrap_or_else(std::env::temp_dir).join("Lamina")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::{Exif, write_jpeg};

    #[test]
    fn importa_ordena_por_data_e_reaproveita_na_segunda_vez() {
        let dir = tempfile::tempdir().unwrap();
        let photos = dir.path().join("fotos");
        std::fs::create_dir_all(&photos).unwrap();
        write_jpeg(
            &photos,
            "b.jpg",
            60,
            40,
            Some(&Exif::new(1, "2026:09:12 16:00:00", "C")),
        );
        write_jpeg(
            &photos,
            "a.jpg",
            60,
            40,
            Some(&Exif::new(1, "2026:09:12 15:00:00", "C")),
        );
        write_jpeg(&photos, "sem_data.jpg", 40, 60, None);
        std::fs::write(photos.join("quebrada.jpg"), b"nada").unwrap();
        let catalog = Catalog::open_in_memory().unwrap();
        let thumbs = dir.path().join("miniaturas");

        let first = import_folder(&photos, &catalog, &thumbs).unwrap();
        assert_eq!(
            (first.processed, first.reused, first.failures.len()),
            (3, 0, 1)
        );
        let names: Vec<_> = first
            .entries
            .iter()
            .map(|e| e.path.file_name().unwrap().to_string_lossy().into_owned())
            .collect();
        assert_eq!(names, ["a.jpg", "b.jpg", "sem_data.jpg"]);
        assert!(first.entries.iter().all(|e| e.thumbnail.exists()));

        let second = import_folder(&photos, &catalog, &thumbs).unwrap();
        assert_eq!((second.processed, second.reused), (0, 3));
    }

    #[test]
    fn heic_e_reportado_como_nao_suportado() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("iphone.heic"), b"x").unwrap();
        let catalog = Catalog::open_in_memory().unwrap();
        let report = import_folder(dir.path(), &catalog, &dir.path().join("t")).unwrap();
        assert_eq!(report.failures.len(), 1);
        assert!(report.failures[0].1.contains("não suportado"));
    }

    #[test]
    fn andamento_cresce_ate_o_total_e_gera_imagens_de_tela() {
        let dir = tempfile::tempdir().unwrap();
        for i in 0..6 {
            write_jpeg(dir.path(), &format!("{i}.jpg"), 60, 40, None);
        }
        let catalog = Catalog::open_in_memory().unwrap();
        let (thumbs, screens) = (dir.path().join("m"), dir.path().join("t"));
        let dirs = DerivedDirs {
            thumbs: &thumbs,
            screens: Some(&screens),
        };
        let seen = Mutex::new(Vec::new());
        let progress = |done: usize, total: usize| seen.lock().unwrap().push((done, total));
        let report = import_folder_with(dir.path(), &catalog, dirs, &progress).unwrap();
        let seen = seen.into_inner().unwrap();
        assert!(seen.windows(2).all(|w| w[0].0 < w[1].0), "{seen:?}");
        assert_eq!(seen.last(), Some(&(6, 6)));
        let fp = &report.entries[0].fingerprint;
        assert!(derived_path(&screens, fp).exists());
    }
}
