//! TR-B12 (spec `travamento-com-muitas-fotos`): o teste de fumaça da importação abre a janela de
//! verdade, com um álbum sem fotos e sem lâminas, importa uma pasta e fecha sozinho.

#![allow(clippy::unwrap_used, clippy::panic)]

use std::path::Path;
use std::process::Command;
use std::time::{Duration, Instant};

const APP: &str = env!("CARGO_BIN_EXE_lamina-app");

/// Roda o app com a configuração e o cache numa pasta própria (nunca os da máquina) e espera no
/// máximo `limit`; devolve o código de saída e a mensagem de erro.
fn run(args: &[&str], home: &Path, limit: Duration) -> (Option<i32>, String) {
    let mut child = Command::new(APP)
        .args(args)
        .env("LOCALAPPDATA", home.join("local"))
        .env("APPDATA", home.join("roaming"))
        .env("HOME", home)
        .env("XDG_CACHE_HOME", home.join("cache"))
        .env("XDG_CONFIG_HOME", home.join("config"))
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    let start = Instant::now();
    loop {
        if let Some(status) = child.try_wait().unwrap() {
            let mut err = String::new();
            if let Some(mut pipe) = child.stderr.take() {
                std::io::Read::read_to_string(&mut pipe, &mut err).unwrap();
            }
            return (status.code(), err);
        }
        if start.elapsed() > limit {
            child.kill().unwrap();
            panic!("{args:?} passou de {limit:?}");
        }
        std::thread::sleep(Duration::from_millis(100));
    }
}

/// Há tela para abrir a janela? (o CI dos PRs roda o Linux sem tela)
fn has_display() -> bool {
    cfg!(any(windows, target_os = "macos"))
        || std::env::var_os("DISPLAY").is_some()
        || std::env::var_os("WAYLAND_DISPLAY").is_some()
}

#[test]
fn tr_b12_verificar_importacao_num_album_vazio() {
    let dir = tempfile::tempdir().unwrap();
    let missing = dir.path().join("nao-existe");
    let (code, err) = run(
        &["--verificar-importacao", missing.to_str().unwrap()],
        dir.path(),
        Duration::from_secs(30),
    );
    assert_eq!(code, Some(1));
    assert!(err.contains("--verificar-importacao:"), "{err:?}");
    if !has_display() {
        return;
    }
    let photos = dir.path().join("fotos");
    std::fs::create_dir_all(&photos).unwrap();
    for i in 0..60u8 {
        let img = image::RgbImage::from_fn(90, 60, |x, y| {
            image::Rgb([i * 4, u8::try_from(x).unwrap(), u8::try_from(y).unwrap()])
        });
        img.save(photos.join(format!("f{i:02}.jpg"))).unwrap();
    }
    let (code, err) = run(
        &["--verificar-importacao", photos.to_str().unwrap()],
        dir.path(),
        Duration::from_secs(120),
    );
    assert_eq!(code, Some(0), "{err}");
    assert!(err.contains("60 fotos no álbum"), "{err}");
    assert!(err.contains("miniaturas na janela"), "{err}");
}
