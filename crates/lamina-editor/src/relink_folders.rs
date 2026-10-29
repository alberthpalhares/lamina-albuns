//! As pastas ausentes (spec RF, regras 1 e 5): o fotógrafo move pastas, não arquivos. As fotos
//! que faltam são agrupadas pela pasta onde estavam, com o disco desconectado apontado.

use std::collections::BTreeMap;
use std::path::{Component, Path, PathBuf};

use lamina_core::{PhotoId, Project};

/// Uma pasta ausente (o termo do glossário): onde as fotos estavam e quantas faltam. As de
/// subpastas contam nela, porque vão juntas quando a pasta é movida.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MissingFolder {
    pub path: PathBuf,
    pub count: usize,
    /// O disco onde a pasta estava, quando ele não está conectado ("E:", "SSD").
    pub disk: Option<String>,
}

impl MissingFolder {
    /// O nome da pasta ("Casamento").
    pub fn name(&self) -> String {
        self.path.file_name().map_or_else(
            || self.path.display().to_string(),
            |n| n.to_string_lossy().into_owned(),
        )
    }

    /// "Casamento · 8 fotos · estava em D:\Fotos", ou "… · no disco E:, que não está conectado".
    pub fn detail(&self) -> String {
        format!("{} · {}", self.name(), self.place())
    }

    /// O mesmo sem o nome: "8 fotos · estava em D:\Fotos" (a linha de baixo da janela).
    pub fn place(&self) -> String {
        let photos = if self.count == 1 {
            "1 foto".to_owned()
        } else {
            format!("{} fotos", self.count)
        };
        let place = match &self.disk {
            Some(disk) => format!("no disco {disk}, que não está conectado"),
            None => {
                let parent = self.path.parent().unwrap_or(&self.path);
                format!("estava em {}", parent.display())
            }
        };
        format!("{photos} · {place}")
    }
}

/// As pastas das fotos `missing`, em ordem de caminho (regra 1).
pub fn missing_folders(project: &Project, missing: &[PhotoId]) -> Vec<MissingFolder> {
    let mut dirs: BTreeMap<PathBuf, usize> = BTreeMap::new();
    for photo in missing.iter().filter_map(|id| project.photo(*id).ok()) {
        let dir = photo.path.parent().unwrap_or(&photo.path).to_path_buf();
        *dirs.entry(dir).or_default() += 1;
    }
    // Em ordem de caminho, uma pasta vem antes das subpastas dela.
    let mut out: Vec<MissingFolder> = Vec::new();
    for (dir, count) in dirs {
        match out.iter_mut().find(|f| dir.starts_with(&f.path)) {
            Some(parent) => parent.count += count,
            None => out.push(MissingFolder {
                disk: disconnected(&dir),
                path: dir,
                count,
            }),
        }
    }
    out
}

/// O disco de um caminho, quando ele não existe agora (regra 5): a letra do Windows ou o volume do
/// Mac e do Linux (`/Volumes/SSD`, `/media/ana/SSD`, `/run/media/ana/SSD`, `/mnt/SSD`).
pub fn disconnected(path: &Path) -> Option<String> {
    let (volume, label) = volume_of(path)?;
    (!volume.exists()).then_some(label)
}

fn volume_of(path: &Path) -> Option<(PathBuf, String)> {
    let mut parts = path.components();
    match parts.next()? {
        Component::Prefix(prefix) => {
            let letter = prefix.as_os_str().to_string_lossy().into_owned();
            Some((PathBuf::from(format!("{letter}\\")), letter))
        }
        Component::RootDir => {
            let names: Vec<String> = parts
                .take(4)
                .map(|c| c.as_os_str().to_string_lossy().into_owned())
                .collect();
            let depth = match names.first().map(String::as_str) {
                Some("Volumes" | "mnt") => 2,
                Some("media") => 3,
                Some("run") if names.get(1).is_some_and(|n| n == "media") => 4,
                _ => return None,
            };
            let mut volume = PathBuf::from("/");
            names.iter().take(depth).for_each(|n| volume.push(n));
            let label = names.get(depth - 1)?.clone();
            Some((volume, label))
        }
        _ => None,
    }
}
