use super::*;
use crate::{AlbumSpec, Dpi, Mm};

fn abs(text: &str) -> PathBuf {
    if cfg!(windows) {
        PathBuf::from(format!("C:{}", text.replace('/', "\\")))
    } else {
        PathBuf::from(text)
    }
}

fn project() -> Project {
    let spec = AlbumSpec::new(Mm(300.0), Mm(300.0), Dpi::new(300).unwrap()).unwrap();
    Project::new("teste", spec)
}

#[test]
fn foto_na_pasta_irma() {
    let rel = relative_path(&abs("/base/saida"), &abs("/base/fotos/a.jpg"));
    assert_eq!(rel.as_deref(), Some("../fotos/a.jpg"));
}

#[test]
fn foto_dentro_da_pasta_do_projeto() {
    let rel = relative_path(&abs("/base"), &abs("/base/fotos/dia 1/a.jpg"));
    assert_eq!(rel.as_deref(), Some("fotos/dia 1/a.jpg"));
}

#[test]
fn mesma_pasta_e_pasta_mae() {
    assert_eq!(
        relative_path(&abs("/base"), &abs("/base")).as_deref(),
        Some(".")
    );
    assert_eq!(
        relative_path(&abs("/base/saida"), &abs("/base")).as_deref(),
        Some("..")
    );
}

#[test]
fn ponto_no_comeco_e_ignorado_e_ponto_ponto_recusa() {
    let rel = relative_path(Path::new("."), Path::new("fotos/a.jpg"));
    assert_eq!(rel.as_deref(), Some("fotos/a.jpg"));
    assert_eq!(relative_path(&abs("/a/b/../c"), &abs("/a/c/x.jpg")), None);
}

#[test]
fn sem_caminho_relativo_entre_absoluto_e_relativo() {
    assert_eq!(relative_path(&abs("/base"), Path::new("fotos/a.jpg")), None);
}

#[cfg(windows)]
#[test]
fn outro_disco_e_prefixo_verbatim_ficam_absolutos() {
    assert_eq!(
        relative_path(Path::new(r"C:\saida"), Path::new(r"D:\fotos\a.jpg")),
        None
    );
    assert_eq!(
        relative_path(Path::new(r"\\?\C:\saida"), Path::new(r"C:\fotos\a.jpg")),
        None
    );
}

#[test]
fn ida_e_volta_relativizar_e_resolver() {
    let mut p = project();
    p.add_photo(abs("/base/fotos/a.jpg"), 300, 200);
    p.relativize(&abs("/base/saida"));
    assert_eq!(p.photos[0].path, PathBuf::from("../fotos/a.jpg"));
    p.resolve_paths(&abs("/base/saida"));
    let resolved = p.photos[0].path.clone();
    assert!(resolved.starts_with(abs("/base/saida")));
    assert!(resolved.ends_with("a.jpg"));
}

#[test]
fn caminho_absoluto_nao_muda_ao_resolver_e_foto_ausente() {
    let mut p = project();
    p.add_photo(abs("/outro/a.jpg"), 300, 200);
    p.resolve_paths(&abs("/base"));
    assert_eq!(p.photos[0].path, abs("/outro/a.jpg"));
    assert!(p.missing_photo(|_| true).is_none());
    let missing = p.missing_photo(|_| false).map(|f| f.path.clone());
    assert_eq!(missing, Some(abs("/outro/a.jpg")));
}

#[test]
fn nomes_de_projeto() {
    for ok in ["fotos", "Casamento Ana & Bruno", "album_2026-10", "Console"] {
        // Falha mostrando o erro exato, se houver.
        validate_project_name(ok).unwrap();
    }
    for bad in [
        "", "  ", "c:x", "a/b", r"a\b", "x?", "nome.", "nome ", "CON", "nul.txt", "a\u{7}",
    ] {
        assert!(
            matches!(validate_project_name(bad), Err(CoreError::InvalidName(_))),
            "{bad:?}"
        );
    }
    let err = validate_project_name("c:x").unwrap_err();
    assert_eq!(
        err.to_string(),
        "Nome de projeto inválido: \"c:x\". Use letras, números, espaços, - e _."
    );
}
