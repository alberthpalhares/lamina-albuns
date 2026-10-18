//! Fontes embutidas (spec TX; ADR 0008, adendo): toda pasta com um `.ttf` ou `.otf` precisa ter o
//! `OFL.txt` ao lado. Roda também no repositório público, onde as fontes vão junto do código.

use std::fs;
use std::path::Path;

/// As pastas, dentro de `dir`, com fonte e sem `OFL.txt`.
pub fn without_license(dir: &Path) -> Vec<String> {
    let mut out = Vec::new();
    walk(dir, &mut out);
    out.sort();
    out
}

fn walk(dir: &Path, out: &mut Vec<String>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    let paths: Vec<_> = entries.flatten().map(|e| e.path()).collect();
    let has_font = paths.iter().any(|p| {
        p.extension()
            .is_some_and(|e| e.eq_ignore_ascii_case("ttf") || e.eq_ignore_ascii_case("otf"))
    });
    if has_font && !dir.join("OFL.txt").is_file() {
        out.push(dir.display().to_string());
    }
    for p in paths.iter().filter(|p| p.is_dir()) {
        if p.file_name().is_some_and(|n| n == "target") {
            continue;
        }
        walk(p, out);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fonte_sem_licenca_e_apontada() {
        let dir = tempfile::tempdir().unwrap();
        let a = dir.path().join("a");
        let b = dir.path().join("b");
        fs::create_dir_all(&a).unwrap();
        fs::create_dir_all(&b).unwrap();
        fs::write(a.join("A.ttf"), b"x").unwrap();
        fs::write(a.join("OFL.txt"), b"licen\xc3\xa7a").unwrap();
        fs::write(b.join("B.ttf"), b"x").unwrap();
        assert_eq!(without_license(dir.path()), [b.display().to_string()]);
    }
}
