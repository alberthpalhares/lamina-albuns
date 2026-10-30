//! Compila as telas `ui/*.slint` (ADR 0002) e, no Windows, põe o ícone e o nome no executável.

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = slint_build::CompilerConfiguration::new().with_style("fluent".into());
    slint_build::compile_with_config("ui/app.slint", config)?;
    news_images()?;
    windows_icon()?;
    Ok(())
}

/// As ilustrações do "O que há de novo" (spec NV, seção 4) vão dentro do programa (regra 11):
/// a lista `(nome, bytes)` de `dist/novidades/`, em ordem de nome.
fn news_images() -> std::io::Result<()> {
    let folder = std::path::Path::new("dist/novidades");
    println!("cargo:rerun-if-changed=dist/novidades");
    let mut names: Vec<String> = match std::fs::read_dir(folder) {
        Ok(dir) => dir
            .filter_map(Result::ok)
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .filter(|n| n.ends_with(".png") || n.ends_with(".jpg"))
            .collect(),
        Err(_) => Vec::new(),
    };
    names.sort();
    let root = std::env::var("CARGO_MANIFEST_DIR")
        .unwrap_or_default()
        .replace('\\', "/");
    let rows: String = names
        .iter()
        .map(|n| format!("    (\"{n}\", include_bytes!(\"{root}/dist/novidades/{n}\")),\n"))
        .collect();
    let out = std::path::PathBuf::from(std::env::var("OUT_DIR").unwrap_or_default());
    std::fs::write(
        out.join("novidades.rs"),
        format!("pub const IMAGES: &[(&str, &[u8])] = &[\n{rows}];\n"),
    )
}

/// O ícone do Lamina (`icons/lamina.ico`, de `docs/marca/gerar.py`) e o nome do produto nas
/// propriedades do `.exe`.
#[cfg(windows)]
fn windows_icon() -> std::io::Result<()> {
    println!("cargo:rerun-if-changed=icons/lamina.ico");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return Ok(());
    }
    let mut res = winresource::WindowsResource::new();
    res.set_icon("icons/lamina.ico")
        .set("ProductName", "Lamina")
        .set("FileDescription", "Lamina");
    res.compile()
}

#[cfg(not(windows))]
fn windows_icon() -> std::io::Result<()> {
    Ok(())
}
