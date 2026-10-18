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
    // A licença precisa ser a OFL de verdade, com o copyright da família (auditoria do Marco 3,
    // M9): um arquivo vazio ou de outra licença não vale.
    let license = fs::read_to_string(dir.join("OFL.txt")).unwrap_or_default();
    let ok = license.contains("SIL Open Font License") && license.contains("Copyright");
    if has_font && !ok {
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
        let ofl = "Copyright 2020 Fulano\nSIL Open Font License, Version 1.1";
        fs::write(a.join("OFL.txt"), ofl).unwrap();
        fs::write(b.join("B.ttf"), b"x").unwrap();
        // Proveta do M9 do Marco 3: um OFL.txt vazio não vale.
        fs::write(b.join("OFL.txt"), "").unwrap();
        assert_eq!(without_license(dir.path()), [b.display().to_string()]);
    }
}
