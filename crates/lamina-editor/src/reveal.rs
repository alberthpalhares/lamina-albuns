//! "Mostrar no Explorador/Finder" (spec TF, regra 7): o comando que abre a pasta da foto com o
//! arquivo selecionado. A janela só executa o que sai daqui.

use std::path::Path;

use crate::EditorError;

/// Para qual sistema montar o comando.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum System {
    Windows,
    Mac,
    Linux,
}

impl System {
    /// O sistema em que o Lamina está rodando.
    pub fn current() -> System {
        if cfg!(target_os = "windows") {
            System::Windows
        } else if cfg!(target_os = "macos") {
            System::Mac
        } else {
            System::Linux
        }
    }

    /// O item do menu de botão direito (regra 1).
    pub fn reveal_label(self) -> &'static str {
        match self {
            System::Windows => "Mostrar no Explorador",
            System::Mac => "Mostrar no Finder",
            System::Linux => "Abrir a pasta",
        }
    }
}

/// O programa e os argumentos que mostram `file`: com o arquivo selecionado no Windows e no Mac;
/// só a pasta no Linux (seção 11). Arquivo que sumiu vira um erro humano (seção 6).
pub fn reveal_command(file: &Path, system: System) -> Result<(String, Vec<String>), EditorError> {
    if !file.is_file() {
        return Err(EditorError::PhotoFileMissing(file.display().to_string()));
    }
    let shown = file.display().to_string();
    Ok(match system {
        System::Windows => ("explorer".to_owned(), vec![format!("/select,{shown}")]),
        System::Mac => ("open".to_owned(), vec!["-R".to_owned(), shown]),
        System::Linux => {
            let folder = file.parent().unwrap_or(Path::new("."));
            ("xdg-open".to_owned(), vec![folder.display().to_string()])
        }
    })
}
