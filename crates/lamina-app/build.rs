//! Compila as telas `ui/*.slint` (ADR 0002) e, no Windows, põe o ícone e o nome no executável.

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = slint_build::CompilerConfiguration::new().with_style("fluent-dark".into());
    slint_build::compile_with_config("ui/app.slint", config)?;
    windows_icon()?;
    Ok(())
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
