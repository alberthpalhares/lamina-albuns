//! Arquivo do projeto (`.lamina`, ADR 0004; spec CB, regras 4, 6 e 7): gravar com as fotos em
//! caminhos relativos à pasta do arquivo, e abrir resolvendo esses caminhos.

use std::io::Write as _;
use std::path::{Path, PathBuf};

use lamina_core::{CoreError, Project, lexical_path, validate_project_name};
use thiserror::Error;

/// Extensão do arquivo de projeto.
pub const PROJECT_EXT: &str = "lamina";

/// Erros ao gravar ou abrir o arquivo do projeto, com as mensagens da spec.
#[derive(Debug, Error)]
pub enum ProjectFileError {
    #[error("Não foi possível ler o projeto {path}: {reason}.")]
    Read { path: String, reason: String },
    #[error("Não foi possível gravar {path}: {reason}.")]
    Write { path: String, reason: String },
    #[error("A foto {name} não foi encontrada em {path}.")]
    MissingPhoto { name: String, path: String },
    #[error(transparent)]
    Core(#[from] CoreError),
}

/// `<nome>.lamina`, se o nome serve de nome de arquivo nos três sistemas.
pub fn project_file_name(name: &str) -> Result<String, CoreError> {
    validate_project_name(name)?;
    Ok(format!("{name}.{PROJECT_EXT}"))
}

/// Caminho absoluto sem tocar o disco (sem o `\\?\` do `canonicalize` no Windows) e com `..`
/// resolvido pelo texto também no Linux e no macOS; se não der, o próprio caminho.
fn absolute(path: &Path) -> PathBuf {
    lexical_path(&std::path::absolute(path).unwrap_or_else(|_| path.to_path_buf()))
}

/// Grava o projeto em `folder/file` com os caminhos das fotos relativos a `folder`, num
/// temporário que vai para o disco e é renomeado no fim.
pub fn save_project(
    project: &Project,
    folder: &Path,
    file: &str,
) -> Result<PathBuf, ProjectFileError> {
    let path = folder.join(file);
    let fail = |reason: String| ProjectFileError::Write {
        path: path.display().to_string(),
        reason,
    };
    let mut saved = project.clone();
    for photo in &mut saved.photos {
        photo.path = absolute(&photo.path);
    }
    saved.relativize(&absolute(folder));
    let json = saved.to_json()?;
    let mut tmp = tempfile::Builder::new()
        .prefix(".lamina-")
        .suffix(".tmp")
        .tempfile_in(folder)
        .map_err(|e| fail(e.to_string()))?;
    tmp.write_all(json.as_bytes())
        .map_err(|e| fail(e.to_string()))?;
    tmp.as_file().sync_all().map_err(|e| fail(e.to_string()))?;
    tmp.persist(&path).map_err(|e| fail(e.error.to_string()))?;
    Ok(path)
}

/// Abre o projeto e resolve os caminhos a partir da pasta do arquivo, sem conferir se as fotos
/// existem (a janela abre mesmo com foto ausente; spec JP, regra 11).
pub fn read_project(file: &Path) -> Result<Project, ProjectFileError> {
    let text = std::fs::read_to_string(file).map_err(|e| ProjectFileError::Read {
        path: file.display().to_string(),
        reason: e.to_string(),
    })?;
    let mut project = Project::from_json(&text)?;
    let folder = absolute(file.parent().unwrap_or(Path::new(".")));
    project.resolve_paths(&folder);
    Ok(project)
}

/// Abre o projeto, resolve os caminhos a partir da pasta do arquivo e confere que as fotos
/// existem.
pub fn open_project(file: &Path) -> Result<Project, ProjectFileError> {
    let project = read_project(file)?;
    if let Some(missing) = project.missing_photo(Path::exists) {
        let name = missing.path.file_name().unwrap_or(missing.path.as_os_str());
        return Err(ProjectFileError::MissingPhoto {
            name: name.to_string_lossy().into_owned(),
            path: missing.path.display().to_string(),
        });
    }
    Ok(project)
}

#[cfg(test)]
mod tests {
    use lamina_core::{AlbumSpec, Dpi, Mm};

    use super::*;

    fn project_with(photo: &Path) -> Project {
        let spec = AlbumSpec::new(Mm(300.0), Mm(300.0), Dpi::new(300).unwrap()).unwrap();
        let mut project = Project::new("teste", spec);
        project.add_photo(photo, 300, 200);
        project
    }

    #[test]
    fn grava_relativo_e_abre_de_novo() {
        let dir = tempfile::tempdir().unwrap();
        let (fotos, saida) = (dir.path().join("fotos"), dir.path().join("saida"));
        std::fs::create_dir_all(&fotos).unwrap();
        std::fs::create_dir_all(&saida).unwrap();
        std::fs::write(fotos.join("a.jpg"), b"x").unwrap();
        let project = project_with(&fotos.join("a.jpg"));
        let file = save_project(&project, &saida, "album.lamina").unwrap();
        let text = std::fs::read_to_string(&file).unwrap();
        assert!(text.contains("\"../fotos/a.jpg\""), "{text}");
        let opened = open_project(&file).unwrap();
        assert!(opened.photos[0].path.exists());
        assert_eq!(opened.spreads, project.spreads);
    }

    /// Achado no IA-B16 (CI do Linux): abrir e salvar de novo, noutra pasta irmã, gravava
    /// `/…/um/../fotos/a.jpg`, absoluto e com `..`, no lugar de `../fotos/a.jpg`.
    #[test]
    fn abrir_e_salvar_de_novo_mantem_o_caminho_relativo() {
        let dir = tempfile::tempdir().unwrap();
        let fotos = dir.path().join("fotos");
        for folder in ["fotos", "um", "dois"] {
            std::fs::create_dir_all(dir.path().join(folder)).unwrap();
        }
        std::fs::write(fotos.join("a.jpg"), b"x").unwrap();
        let first = save_project(
            &project_with(&fotos.join("a.jpg")),
            &dir.path().join("um"),
            "a.lamina",
        )
        .unwrap();
        let opened = read_project(&first).unwrap();
        assert_eq!(opened.photos[0].path, absolute(&fotos.join("a.jpg")));
        let again = save_project(&opened, &dir.path().join("dois"), "a.lamina").unwrap();
        let text = std::fs::read_to_string(&again).unwrap();
        assert!(text.contains("\"../fotos/a.jpg\""), "{text}");
        assert_eq!(text, std::fs::read_to_string(&first).unwrap());
    }

    #[test]
    fn foto_que_sumiu_e_erro() {
        let dir = tempfile::tempdir().unwrap();
        let project = project_with(&dir.path().join("sumiu.jpg"));
        let file = save_project(&project, dir.path(), "album.lamina").unwrap();
        let err = open_project(&file).unwrap_err();
        assert!(matches!(&err, ProjectFileError::MissingPhoto { name, .. } if name == "sumiu.jpg"));
        assert!(
            err.to_string()
                .starts_with("A foto sumiu.jpg não foi encontrada em ")
        );
    }

    #[test]
    fn arquivo_invalido_e_nome_invalido() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("ruim.lamina");
        std::fs::write(&file, "isto não é json").unwrap();
        assert!(matches!(
            open_project(&file),
            Err(ProjectFileError::Core(CoreError::Json(_)))
        ));
        let missing = dir.path().join("nao-existe.lamina");
        assert!(matches!(
            open_project(&missing),
            Err(ProjectFileError::Read { .. })
        ));
        assert_eq!(project_file_name("fotos").unwrap(), "fotos.lamina");
        assert!(matches!(
            project_file_name("c:x"),
            Err(CoreError::InvalidName(_))
        ));
    }
}
