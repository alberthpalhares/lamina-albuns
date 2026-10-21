//! DS-B17 e B18 (spec `distribuicao`): os scripts de instalação pelo terminal com uma release
//! de mentira numa pasta (`LAMINA_FONTE`). No Mac e no Linux o `instalar.sh` instala de verdade
//! num `HOME` temporário; no Windows o `instalar.ps1` para depois de conferir (`LAMINA_TESTE=1`),
//! porque o instalador de verdade só existe na release (o teste de fumaça dela roda o resto).

#![allow(clippy::unwrap_used)]

use std::path::Path;
use std::process::{Command, Output};

const MISMATCH: &str = "O arquivo baixado não confere; nada foi instalado.";

fn script(name: &str) -> String {
    format!("{}/dist/{name}", env!("CARGO_MANIFEST_DIR"))
}

/// A soma SHA-256 pelo programa do sistema (o mesmo que o script usa).
fn sha256(file: &Path) -> String {
    let out = if cfg!(windows) {
        let cmd = format!(
            "(Get-FileHash -LiteralPath '{}' -Algorithm SHA256).Hash",
            file.display()
        );
        Command::new("pwsh")
            .args(["-NoProfile", "-Command", &cmd])
            .output()
            .unwrap()
    } else if cfg!(target_os = "macos") {
        Command::new("shasum")
            .args(["-a", "256"])
            .arg(file)
            .output()
            .unwrap()
    } else {
        Command::new("sha256sum").arg(file).output().unwrap()
    };
    let text = String::from_utf8(out.stdout).unwrap();
    text.split_whitespace().next().unwrap().to_lowercase()
}

/// A release de mentira: o pacote do sistema e o `SHA256SUMS` (com a soma certa ou não).
fn fake_release(dir: &Path, right_sum: bool) -> String {
    let name = if cfg!(windows) {
        std::fs::write(dir.join("Lamina-win-Setup.exe"), b"instalador de mentira").unwrap();
        "Lamina-win-Setup.exe"
    } else if cfg!(target_os = "macos") {
        let app = dir.join("monta/Lamina.app/Contents/MacOS");
        std::fs::create_dir_all(&app).unwrap();
        std::fs::write(app.join("lamina-app"), b"#!/bin/sh\necho lamina\n").unwrap();
        let ok = Command::new("ditto")
            .args(["-c", "-k", "--keepParent"])
            .arg(dir.join("monta/Lamina.app"))
            .arg(dir.join("Lamina-osx-Portable.zip"))
            .status()
            .unwrap();
        assert!(ok.success());
        "Lamina-osx-Portable.zip"
    } else {
        std::fs::write(dir.join("Lamina.AppImage"), b"#!/bin/sh\necho lamina\n").unwrap();
        "Lamina.AppImage"
    };
    let sum = if right_sum {
        sha256(&dir.join(name))
    } else {
        "0".repeat(64)
    };
    std::fs::write(
        dir.join("SHA256SUMS"),
        format!("{}  outro-sistema.zip\n{sum}  {name}\n", "1".repeat(64)),
    )
    .unwrap();
    name.to_owned()
}

fn install(release: &Path, home: &Path) -> Output {
    if cfg!(windows) {
        Command::new("pwsh")
            .args(["-NoProfile", "-ExecutionPolicy", "Bypass", "-File"])
            .arg(script("instalar.ps1"))
            .env("LAMINA_FONTE", release)
            .env("LAMINA_TESTE", "1")
            .output()
            .unwrap()
    } else {
        // A pasta Aplicativos do Mac de mentira não existe: o script cai na pasta pessoal (spec MA).
        Command::new("sh")
            .arg(script("instalar.sh"))
            .env("LAMINA_FONTE", release)
            .env("LAMINA_APLICATIVOS", home.join("nao-existe"))
            .env("HOME", home)
            .output()
            .unwrap()
    }
}

fn installed(home: &Path) -> std::path::PathBuf {
    if cfg!(target_os = "macos") {
        home.join("Applications/Lamina.app/Contents/MacOS/lamina-app")
    } else {
        home.join(".local/bin/Lamina.AppImage")
    }
}

#[test]
fn ds_b17_instalar_pelo_terminal() {
    let (release, home) = (tempfile::tempdir().unwrap(), tempfile::tempdir().unwrap());
    let name = fake_release(release.path(), true);
    let out = install(release.path(), home.path());
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(
        out.status.success(),
        "{text}{}",
        String::from_utf8_lossy(&out.stderr)
    );
    if cfg!(windows) {
        assert!(text.contains(&format!("conferido: {name}")), "{text}");
    } else {
        let app = installed(home.path());
        assert!(app.exists(), "{text}");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt as _;
            let mode = std::fs::metadata(&app).unwrap().permissions().mode();
            assert!(mode & 0o111 != 0, "executável");
        }
        assert!(text.contains("Pronto!"), "{text}");
    }
}

#[test]
fn ds_b18_soma_que_nao_bate() {
    let (release, home) = (tempfile::tempdir().unwrap(), tempfile::tempdir().unwrap());
    fake_release(release.path(), false);
    let out = install(release.path(), home.path());
    assert_eq!(out.status.code(), Some(1));
    let all = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(all.contains(MISMATCH), "{all}");
    assert!(!all.contains("conferido"), "{all}");
    assert!(!installed(home.path()).exists(), "nada instalado");
}
