//! Reencontrar a pasta que mudou de lugar (spec RF, regras 3 e 4): o mesmo caminho dentro da
//! pasta nova, depois o mesmo nome, e só então o conteúdo de todas. Nos dois primeiros passos só
//! os candidatos são lidos, um arquivo por foto ausente.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use lamina_core::{PhotoId, Project};
use lamina_media::fingerprint::fingerprint;
use lamina_media::{list_photos, read_metadata};

use crate::EditorError;
use crate::relink::{Found, index_by_name};

/// Lê a impressão digital de um arquivo (nos testes, contando as leituras).
pub type Print<'a> = &'a dyn Fn(&Path) -> Option<String>;

/// Uma foto ausente: o caminho antigo e a impressão digital que prova que é ela.
struct Wanted {
    id: PhotoId,
    path: PathBuf,
    print: String,
}

/// Procura as fotos `missing` na pasta `to`. Com `from` (a pasta antiga que o fotógrafo apontou),
/// o passo 1 troca `from` por `to` e a mesma troca vale para as outras pastas ausentes.
pub fn search_moved(
    project: &Project,
    missing: &[PhotoId],
    from: Option<&Path>,
    to: &Path,
    progress: &dyn Fn(usize, usize),
) -> Vec<Found> {
    search_moved_with(project, missing, (from, to), progress, &|p| {
        #[cfg(test)]
        READS.with(|r| r.set(r.get() + 1));
        fingerprint(p).ok()
    })
}

#[cfg(test)]
thread_local! {
    /// Quantos arquivos tiveram o conteúdo lido nesta linha de execução (RF-B3 e RF-B11).
    static READS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

/// Zera e devolve a conta de leituras (só nos testes).
#[cfg(test)]
pub(crate) fn take_reads() -> usize {
    READS.with(|r| r.replace(0))
}

pub(crate) fn search_moved_with(
    project: &Project,
    missing: &[PhotoId],
    (from, to): (Option<&Path>, &Path),
    progress: &dyn Fn(usize, usize),
    print: Print<'_>,
) -> Vec<Found> {
    let roots = from.map(|f| rebase(f, to)).unwrap_or_default();
    let mut found = Vec::new();
    let mut left = Vec::new();
    for w in wanted(project, missing) {
        let same_place = roots.iter().find_map(|(old, new)| {
            let rel = w.path.strip_prefix(old).ok()?;
            confirm(&w, &new.join(rel), print)
        });
        match same_place {
            Some(f) => found.push(f),
            None => left.push(w),
        }
    }
    let by_name = index_by_name(to);
    let mut deep = Vec::new();
    for w in left {
        let name = w.path.file_name().map(|n| n.to_string_lossy().into_owned());
        let near = name.and_then(|n| by_name.get(&n));
        match near.and_then(|paths| paths.iter().find_map(|p| confirm(&w, p, print))) {
            Some(f) => found.push(f),
            None if from.is_none_or(|f| w.path.starts_with(f)) => deep.push(w),
            None => {}
        }
    }
    found.extend(by_content(&deep, to, &found, progress, print));
    found
}

/// O passo 3: o conteúdo de todas as fotos da pasta, só quando o nome não bastou.
fn by_content(
    wanted: &[Wanted],
    folder: &Path,
    already: &[Found],
    progress: &dyn Fn(usize, usize),
    print: Print<'_>,
) -> Vec<Found> {
    if wanted.is_empty() {
        return Vec::new();
    }
    let taken: HashSet<&Path> = already.iter().map(|f| f.path.as_path()).collect();
    let files: Vec<PathBuf> = list_photos(folder)
        .unwrap_or_default()
        .into_iter()
        .filter(|p| !taken.contains(p.as_path()))
        .collect();
    let mut out: Vec<Found> = Vec::new();
    for (i, path) in files.iter().enumerate() {
        progress(i + 1, files.len());
        let Some(fp) = print(path) else { continue };
        let hit = wanted
            .iter()
            .find(|w| w.print == fp && !out.iter().any(|f| f.id == w.id));
        if let Some(w) = hit {
            out.extend(measured(w.id, path, None));
        }
    }
    out
}

/// As raízes trocadas: `from` → `to` e, tirando o final comum dos dois caminhos, a raiz antiga
/// → a nova (`D:\Fotos\Casamento` em `E:\Fotos\Casamento` dá `D:\` → `E:\`).
fn rebase(from: &Path, to: &Path) -> Vec<(PathBuf, PathBuf)> {
    let mut roots = vec![(from.to_path_buf(), to.to_path_buf())];
    let (mut old, mut new) = (from.to_path_buf(), to.to_path_buf());
    loop {
        let same = match (old.file_name(), new.file_name()) {
            (Some(a), Some(b)) => {
                a.to_string_lossy().to_lowercase() == b.to_string_lossy().to_lowercase()
            }
            _ => false,
        };
        if !same || !old.pop() || !new.pop() {
            break;
        }
    }
    if (old.as_path(), new.as_path()) != (from, to) {
        roots.push((old, new));
    }
    roots
}

/// O arquivo é a foto? Então ela volta, com as medidas relidas (EI, regra 7).
fn confirm(w: &Wanted, path: &Path, print: Print<'_>) -> Option<Found> {
    if !path.is_file() || print(path)? != w.print {
        return None;
    }
    measured(w.id, path, None)
}

fn measured(id: PhotoId, path: &Path, print: Option<String>) -> Option<Found> {
    let meta = read_metadata(path).ok()?;
    Some(Found {
        id,
        path: path.to_path_buf(),
        width: meta.width,
        height: meta.height,
        fingerprint: print,
    })
}

fn wanted(project: &Project, missing: &[PhotoId]) -> Vec<Wanted> {
    missing
        .iter()
        .filter_map(|id| project.photo(*id).ok())
        .filter_map(|p| {
            Some(Wanted {
                id: p.id,
                path: p.path.clone(),
                print: p.fingerprint.clone()?,
            })
        })
        .collect()
}

/// O que "Localizar esta foto…" achou (regra 4).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Located {
    /// O mesmo conteúdo: a foto e as vizinhas da mesma pasta antiga.
    Same(Vec<Found>),
    /// Outro conteúdo: só com o "Usar mesmo assim" do fotógrafo.
    Different(Found),
}

/// A foto `id` no arquivo `file`. Um arquivo que não abre como foto é recusado.
pub fn locate(
    project: &Project,
    missing: &[PhotoId],
    id: PhotoId,
    file: &Path,
) -> Result<Located, EditorError> {
    let photo = project.photo(id)?;
    let meta = read_metadata(file)?;
    let print = fingerprint(file)?;
    if photo.fingerprint.as_deref() != Some(print.as_str()) {
        return Ok(Located::Different(Found {
            id,
            path: file.to_path_buf(),
            width: meta.width,
            height: meta.height,
            fingerprint: Some(print),
        }));
    }
    let mut found = vec![Found {
        id,
        path: file.to_path_buf(),
        width: meta.width,
        height: meta.height,
        fingerprint: None,
    }];
    let others: Vec<PhotoId> = missing.iter().copied().filter(|m| *m != id).collect();
    if let (Some(old), Some(new)) = (photo.path.parent(), file.parent()) {
        found.extend(search_moved(project, &others, Some(old), new, &|_, _| {}));
    }
    Ok(Located::Same(found))
}
