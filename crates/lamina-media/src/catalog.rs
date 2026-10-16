//! Catálogo de fotos em SQLite (ADR 0004).
//!
//! Guarda, para cada arquivo, os metadados e a miniatura já calculados. Na próxima importação,
//! um arquivo com o mesmo tamanho e a mesma data de modificação é reaproveitado sem ser lido.
//! O catálogo é um cache: apagá-lo nunca perde trabalho.

use std::path::{Path, PathBuf};

use lamina_core::CaptureTime;
use rusqlite::{Connection, OptionalExtension, Row, params};

use crate::MediaError;

/// Versão do esquema do catálogo (`PRAGMA user_version`).
const SCHEMA_VERSION: i64 = 1;

/// Uma foto conhecida pelo catálogo.
#[derive(Debug, Clone, PartialEq)]
pub struct CatalogEntry {
    pub path: PathBuf,
    pub fingerprint: String,
    /// Tamanho do arquivo em bytes e data de modificação (segundos desde 1970), para saber se
    /// o arquivo mudou.
    pub file_size: i64,
    pub modified: i64,
    pub width: u32,
    pub height: u32,
    pub exif_orientation: u8,
    pub taken_at: Option<CaptureTime>,
    pub camera: Option<String>,
    pub thumbnail: PathBuf,
}

/// Conexão com o catálogo.
#[derive(Debug)]
pub struct Catalog {
    conn: Connection,
}

impl Catalog {
    /// Abre (ou cria) o catálogo no arquivo `path`.
    pub fn open(path: &Path) -> Result<Self, MediaError> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir).map_err(MediaError::io(dir))?;
        }
        Self::init(Connection::open(path)?)
    }

    /// Catálogo só em memória (testes).
    pub fn open_in_memory() -> Result<Self, MediaError> {
        Self::init(Connection::open_in_memory()?)
    }

    fn init(conn: Connection) -> Result<Self, MediaError> {
        let version: i64 = conn.pragma_query_value(None, "user_version", |r| r.get(0))?;
        if version < SCHEMA_VERSION {
            conn.execute_batch(
                "CREATE TABLE IF NOT EXISTS photos (
                    path TEXT PRIMARY KEY,
                    fingerprint TEXT NOT NULL,
                    file_size INTEGER NOT NULL,
                    modified INTEGER NOT NULL,
                    width INTEGER NOT NULL,
                    height INTEGER NOT NULL,
                    exif_orientation INTEGER NOT NULL,
                    taken_at INTEGER,
                    camera TEXT,
                    thumbnail TEXT NOT NULL
                );
                CREATE INDEX IF NOT EXISTS photos_fingerprint ON photos(fingerprint);
                PRAGMA user_version = 1;",
            )?;
        }
        Ok(Catalog { conn })
    }

    /// Insere ou atualiza uma foto.
    pub fn upsert(&self, e: &CatalogEntry) -> Result<(), MediaError> {
        self.conn.execute(
            "INSERT INTO photos (path, fingerprint, file_size, modified, width, height,
                                 exif_orientation, taken_at, camera, thumbnail)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
             ON CONFLICT(path) DO UPDATE SET
                fingerprint = excluded.fingerprint, file_size = excluded.file_size,
                modified = excluded.modified, width = excluded.width, height = excluded.height,
                exif_orientation = excluded.exif_orientation, taken_at = excluded.taken_at,
                camera = excluded.camera, thumbnail = excluded.thumbnail",
            params![
                path_text(&e.path),
                e.fingerprint,
                e.file_size,
                e.modified,
                e.width,
                e.height,
                e.exif_orientation,
                e.taken_at.map(CaptureTime::seconds),
                e.camera,
                path_text(&e.thumbnail),
            ],
        )?;
        Ok(())
    }

    /// A foto registrada neste caminho, se houver.
    pub fn get(&self, path: &Path) -> Result<Option<CatalogEntry>, MediaError> {
        let sql = format!("{SELECT} WHERE path = ?1");
        Ok(self
            .conn
            .query_row(&sql, [path_text(path)], from_row)
            .optional()?)
    }

    /// Todas as fotos com esta impressão digital (a mesma foto em pastas diferentes).
    pub fn find_by_fingerprint(&self, fingerprint: &str) -> Result<Vec<CatalogEntry>, MediaError> {
        let sql = format!("{SELECT} WHERE fingerprint = ?1 ORDER BY path");
        let mut stmt = self.conn.prepare(&sql)?;
        let rows = stmt.query_map([fingerprint], from_row)?;
        Ok(rows.collect::<Result<_, _>>()?)
    }

    /// Todas as fotos, em ordem de captura (sem data por último).
    pub fn all(&self) -> Result<Vec<CatalogEntry>, MediaError> {
        let sql = format!("{SELECT} ORDER BY taken_at IS NULL, taken_at, path");
        let mut stmt = self.conn.prepare(&sql)?;
        let rows = stmt.query_map([], from_row)?;
        Ok(rows.collect::<Result<_, _>>()?)
    }
}

const SELECT: &str = "SELECT path, fingerprint, file_size, modified, width, height,
                             exif_orientation, taken_at, camera, thumbnail FROM photos";

fn path_text(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

fn from_row(row: &Row<'_>) -> rusqlite::Result<CatalogEntry> {
    let taken_at: Option<i64> = row.get(7)?;
    Ok(CatalogEntry {
        path: PathBuf::from(row.get::<_, String>(0)?),
        fingerprint: row.get(1)?,
        file_size: row.get(2)?,
        modified: row.get(3)?,
        width: row.get(4)?,
        height: row.get(5)?,
        exif_orientation: row.get(6)?,
        taken_at: taken_at.map(CaptureTime::from_seconds),
        camera: row.get(8)?,
        thumbnail: PathBuf::from(row.get::<_, String>(9)?),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(path: &str, fp: &str, taken: Option<&str>) -> CatalogEntry {
        CatalogEntry {
            path: PathBuf::from(path),
            fingerprint: fp.to_owned(),
            file_size: 1000,
            modified: 1_700_000_000,
            width: 6000,
            height: 4000,
            exif_orientation: 1,
            taken_at: taken.and_then(CaptureTime::from_exif),
            camera: Some("Camera".to_owned()),
            thumbnail: PathBuf::from(format!("cache/{fp}.jpg")),
        }
    }

    #[test]
    fn grava_e_le_de_volta() {
        let catalog = Catalog::open_in_memory().unwrap();
        let e = entry("fotos/a.jpg", "aaa", Some("2026:09:12 15:00:00"));
        catalog.upsert(&e).unwrap();
        assert_eq!(catalog.get(Path::new("fotos/a.jpg")).unwrap(), Some(e));
        assert_eq!(catalog.get(Path::new("fotos/nada.jpg")).unwrap(), None);
    }

    #[test]
    fn upsert_atualiza_em_vez_de_duplicar() {
        let catalog = Catalog::open_in_memory().unwrap();
        catalog.upsert(&entry("a.jpg", "v1", None)).unwrap();
        catalog.upsert(&entry("a.jpg", "v2", None)).unwrap();
        let all = catalog.all().unwrap();
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].fingerprint, "v2");
    }

    #[test]
    fn ordena_por_data_de_captura_com_sem_data_no_fim() {
        let catalog = Catalog::open_in_memory().unwrap();
        catalog.upsert(&entry("sem.jpg", "c", None)).unwrap();
        catalog
            .upsert(&entry("tarde.jpg", "b", Some("2026:09:12 16:00:00")))
            .unwrap();
        catalog
            .upsert(&entry("cedo.jpg", "a", Some("2026:09:12 15:00:00")))
            .unwrap();
        let names: Vec<_> = catalog.all().unwrap().into_iter().map(|e| e.path).collect();
        assert_eq!(
            names,
            ["cedo.jpg", "tarde.jpg", "sem.jpg"].map(PathBuf::from)
        );
    }

    #[test]
    fn acha_a_mesma_foto_em_outra_pasta_pela_impressao_digital() {
        let catalog = Catalog::open_in_memory().unwrap();
        catalog
            .upsert(&entry("antiga/a.jpg", "mesma", None))
            .unwrap();
        catalog.upsert(&entry("nova/a.jpg", "mesma", None)).unwrap();
        catalog
            .upsert(&entry("outra.jpg", "diferente", None))
            .unwrap();
        assert_eq!(catalog.find_by_fingerprint("mesma").unwrap().len(), 2);
    }

    #[test]
    fn reabre_o_arquivo_do_catalogo() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("sub").join("catalogo.sqlite");
        Catalog::open(&file)
            .unwrap()
            .upsert(&entry("a.jpg", "x", None))
            .unwrap();
        assert_eq!(Catalog::open(&file).unwrap().all().unwrap().len(), 1);
    }
}
