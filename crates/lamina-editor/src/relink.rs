//! Reencontrar fotos que mudaram de pasta (spec EI, regras 5 a 7; ADR 0004): um arquivo só vale
//! se tiver a mesma impressão digital (mesmo nome com outro conteúdo não serve).

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use lamina_core::{PhotoId, Project};
use lamina_media::fingerprint::fingerprint;
use lamina_media::{list_photos, read_metadata};

use crate::Cache;

/// Uma foto ausente achada: o caminho novo e as medidas lidas do arquivo (regra 7).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Found {
    pub id: PhotoId,
    pub path: PathBuf,
    pub width: u32,
    pub height: u32,
    /// A impressão digital nova, quando o fotógrafo aceitou um arquivo de outro conteúdo (spec RF,
    /// regra 4); `None` = a mesma de antes.
    pub fingerprint: Option<String>,
}

/// Ao abrir (regra 5): cada foto ausente é procurada (a) pelo mesmo nome na pasta do projeto e
/// nas subpastas, em ordem de caminho, e (b) nos caminhos que o catálogo conhece para a mesma
/// impressão digital. Lê pastas e arquivos: a janela chama fora da linha dela.
pub fn search_near(
    project: &Project,
    missing: &[PhotoId],
    file: Option<&Path>,
    cache: &Cache,
) -> Vec<Found> {
    let by_name = file
        .and_then(Path::parent)
        .map(index_by_name)
        .unwrap_or_default();
    let catalog = cache.open_catalog().ok();
    wanted(project, missing)
        .into_iter()
        .filter_map(|(id, name, fp)| {
            let near = by_name.get(&name).into_iter().flatten().cloned();
            let known = catalog
                .iter()
                .flat_map(|c| c.find_by_fingerprint(&fp).unwrap_or_default())
                .map(|entry| entry.path);
            near.chain(known).find_map(|path| check(id, &path, &fp))
        })
        .collect()
}

/// "Reencontrar fotos…" (regra 6): todas as fotos da pasta e das subpastas, com qualquer
/// nome, comparadas pela impressão digital. `progress` recebe (lidas, total).
pub fn search_folder(
    project: &Project,
    missing: &[PhotoId],
    folder: &Path,
    progress: &dyn Fn(usize, usize),
) -> Vec<Found> {
    let files = list_photos(folder).unwrap_or_default();
    let total = files.len();
    let mut by_print: HashMap<String, PathBuf> = HashMap::new();
    for (i, path) in files.into_iter().enumerate() {
        if let Ok(fp) = fingerprint(&path) {
            // Duas cópias: fica a primeira em ordem de caminho.
            by_print.entry(fp).or_insert(path);
        }
        progress(i + 1, total);
    }
    wanted(project, missing)
        .into_iter()
        .filter_map(|(id, _, fp)| by_print.get(&fp).and_then(|p| check(id, p, &fp)))
        .collect()
}

/// "1 foto reencontrada." ou "3 fotos reencontradas; 2 ainda faltam."; nada se nenhuma voltou.
pub fn relinked_message(found: usize, still_missing: usize) -> Option<String> {
    let mut text = match found {
        0 => return None,
        1 => "1 foto reencontrada".to_owned(),
        n => format!("{n} fotos reencontradas"),
    };
    match still_missing {
        0 => {}
        1 => text.push_str("; 1 ainda falta"),
        n => text.push_str(&format!("; {n} ainda faltam")),
    }
    text.push('.');
    Some(text)
}

/// A procura ao abrir achou fotos, mas não aplicou para não apagar o "refazer".
pub const RELINK_LATER: &str = "Algumas fotos ausentes foram achadas perto do projeto: use \
     Arquivo → Reencontrar fotos… para reencontrá-las.";

/// "Nenhuma das 3 fotos ausentes foi encontrada em <pasta>." (spec EI, seção 6).
pub fn not_found_message(missing: usize, folder: &Path) -> String {
    if missing == 1 {
        format!("A foto ausente não foi encontrada em {}.", folder.display())
    } else {
        format!(
            "Nenhuma das {missing} fotos ausentes foi encontrada em {}.",
            folder.display()
        )
    }
}

/// (id, nome do arquivo, impressão digital) das fotos ausentes. Sem impressão digital não há
/// como conferir o conteúdo, então a foto fica de fora.
fn wanted(project: &Project, missing: &[PhotoId]) -> Vec<(PhotoId, String, String)> {
    missing
        .iter()
        .filter_map(|id| project.photo(*id).ok())
        .filter_map(|photo| {
            let name = photo.path.file_name()?.to_string_lossy().into_owned();
            Some((photo.id, name, photo.fingerprint.clone()?))
        })
        .collect()
}

/// As fotos da pasta e das subpastas pelo nome do arquivo, cada lista em ordem de caminho.
pub(crate) fn index_by_name(folder: &Path) -> HashMap<String, Vec<PathBuf>> {
    let mut index: HashMap<String, Vec<PathBuf>> = HashMap::new();
    for path in list_photos(folder).unwrap_or_default() {
        if let Some(name) = path.file_name() {
            let name = name.to_string_lossy().into_owned();
            index.entry(name).or_default().push(path);
        }
    }
    index
}

/// O arquivo é a foto (mesma impressão digital)? Então as medidas vêm dele.
fn check(id: PhotoId, path: &Path, fp: &str) -> Option<Found> {
    if fingerprint(path).ok()? != fp {
        return None;
    }
    let meta = read_metadata(path).ok()?;
    Some(Found {
        id,
        path: path.to_path_buf(),
        width: meta.width,
        height: meta.height,
        fingerprint: None,
    })
}
