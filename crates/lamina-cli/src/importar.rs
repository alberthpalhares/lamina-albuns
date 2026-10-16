//! Comando `lamina importar`: importa uma pasta e mostra o que o Lâmina entendeu de cada foto.

use std::path::{Path, PathBuf};
use std::time::Instant;

use anyhow::{Context, Result};
use lamina_core::Orientation;
use lamina_editor::Cache;
pub use lamina_editor::project_from;
use lamina_media::{CatalogEntry, ImportReport, import_folder};

/// O cache pedido ou o do sistema (o mesmo da janela).
pub fn cache_at(cache: Option<PathBuf>) -> Cache {
    cache.map_or_else(Cache::system, Cache::new)
}

/// Importa `folder` usando o catálogo e as miniaturas em `cache` (padrão: cache do sistema).
pub fn import(folder: &Path, cache: Option<PathBuf>) -> Result<(ImportReport, PathBuf)> {
    let cache = cache_at(cache);
    let catalog_path = cache.catalog_path();
    let catalog = cache.open_catalog().with_context(|| {
        format!(
            "Não foi possível abrir o catálogo {}.",
            catalog_path.display()
        )
    })?;
    let report = import_folder(folder, &catalog, &cache.thumbs_dir())
        .with_context(|| format!("Não foi possível importar {}.", folder.display()))?;
    Ok((report, cache.root().to_path_buf()))
}

pub fn run(folder: &Path, cache: Option<PathBuf>) -> Result<()> {
    let started = Instant::now();
    let (report, cache) = import(folder, cache)?;
    let elapsed = started.elapsed();
    let catalog_path = cache.join("catalogo.sqlite");
    let thumbs = cache.join("miniaturas");

    println!(
        "{:<28} {:>11}  {:<3} {:<19}  câmera",
        "foto", "pixels", "ori", "captura"
    );
    for entry in &report.entries {
        print_row(folder, entry);
    }
    for (path, reason) in &report.failures {
        println!("  ✘ {}: {reason}", relative(folder, path));
    }
    println!(
        "\n{} foto(s) em {:.2} s: {} lida(s) agora, {} do catálogo, {} com problema.",
        report.entries.len(),
        elapsed.as_secs_f64(),
        report.processed,
        report.reused,
        report.failures.len()
    );
    println!("Catálogo: {}", catalog_path.display());
    println!("Miniaturas: {}", thumbs.display());
    Ok(())
}

fn print_row(folder: &Path, e: &CatalogEntry) {
    let ratio = f64::from(e.width) / f64::from(e.height.max(1));
    let orientation = match Orientation::from_aspect_ratio(ratio) {
        Orientation::Landscape => "H",
        Orientation::Portrait => "V",
        Orientation::Square => "Q",
    };
    let taken = e.taken_at.map_or_else(|| "—".to_owned(), |t| t.to_string());
    println!(
        "{:<28} {:>11}  {:<3} {:<19}  {}",
        relative(folder, &e.path),
        format!("{}×{}", e.width, e.height),
        orientation,
        taken,
        e.camera.as_deref().unwrap_or("—")
    );
}

pub fn relative(folder: &Path, path: &Path) -> String {
    path.strip_prefix(folder)
        .unwrap_or(path)
        .display()
        .to_string()
}
