//! Funções comuns aos testes do binário `lamina` (cenários CB).

// Arquivo só de testes (`tests/`): as funções auxiliares também podem usar `unwrap` (AGENTS.md,
// regra 3, vale fora de testes).
#![allow(
    clippy::unwrap_used,
    clippy::cast_possible_truncation,
    dead_code,
    unused_imports
)]

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use image::{Rgb, RgbImage};
use rayon::prelude::*;

/// Pasta temporária com `fotos/`, `cache/` e espaço para as saídas.
pub struct Base {
    dir: tempfile::TempDir,
}

impl Base {
    /// `count` fotos `w × h` (uma vertical a cada quatro), diferentes entre si. O degradê é
    /// desenhado uma vez só; cada foto muda só um bloco do canto (desenhar pixel a pixel em
    /// código de teste, sem otimização, é lento).
    pub fn with_photos(count: usize, (w, h): (u32, u32)) -> Self {
        let dir = tempfile::tempdir().unwrap();
        let photos = dir.path().join("fotos");
        std::fs::create_dir_all(&photos).unwrap();
        let gradient = |pw: u32, ph: u32| {
            RgbImage::from_fn(pw, ph, |x, y| {
                Rgb([(x * 255 / pw) as u8, (y * 255 / ph) as u8, 128])
            })
        };
        let (landscape, portrait) = (gradient(w, h), gradient(h, w));
        // Codificar JPEGs grandes é o que demora: em paralelo.
        (0..count).into_par_iter().for_each(|i| {
            let mut img = if i % 4 == 3 {
                portrait.clone()
            } else {
                landscape.clone()
            };
            let tone = u8::try_from(i * 37 % 256).unwrap();
            for y in 0..img.height().min(16) {
                for x in 0..img.width().min(16) {
                    img.put_pixel(x, y, Rgb([tone, 255 - tone, tone]));
                }
            }
            img.save(photos.join(format!("f{i:02}.jpg"))).unwrap();
        });
        Base { dir }
    }

    pub fn path(&self, rel: &str) -> PathBuf {
        self.dir.path().join(rel)
    }

    /// Roda `lamina <args>` com o cache dentro da pasta temporária.
    pub fn lamina(&self, args: &[&str]) -> Output {
        self.lamina_in(self.dir.path(), args)
    }

    pub fn lamina_in(&self, cwd: &Path, args: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_lamina"))
            .current_dir(cwd)
            .args(args)
            .output()
            .unwrap()
    }

    /// `lamina build fotos --saida <saida> --cache cache <extra>`, em lâminas pequenas (5 × 5 cm a
    /// 72 DPI) para o teste ser rápido, salvo quando `extra` traz outras medidas.
    pub fn build(&self, saida: &str, extra: &[&str]) -> Output {
        let cache = self.path("cache");
        let mut args = vec!["build", "fotos", "--saida", saida, "--cache"];
        args.push(cache.to_str().unwrap());
        if !extra.iter().any(|a| a.starts_with("--largura")) {
            args.extend(["--largura-cm", "5", "--altura-cm", "5", "--dpi", "72"]);
        }
        args.extend(extra);
        self.lamina(&args)
    }
}

pub fn text(out: &[u8]) -> String {
    String::from_utf8_lossy(out).into_owned()
}

pub fn files(dir: &Path) -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(dir)
        .map(|r| {
            r.map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
                .collect()
        })
        .unwrap_or_default();
    names.sort();
    names
}

pub use lamina_core::testing::budget;
