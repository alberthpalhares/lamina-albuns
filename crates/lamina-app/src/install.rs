//! A associação do `.lamina` (spec DS, regra 10). O que registrar vem do
//! `lamina_editor::updates::association`; aqui só executa: o `reg.exe` do Windows nos ganchos
//! do Velopack e, no Linux, os arquivos do atalho na primeira abertura do AppImage.

#[cfg(windows)]
mod windows {
    use std::os::windows::process::CommandExt as _;
    use std::process::Command;

    use lamina_editor::updates::association::{windows_entries, windows_keys_to_remove};
    use velopack::UpdateManager;
    use velopack::sources::NoneSource;

    /// Sem janela de console piscando durante a instalação.
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;

    /// Fora de uma instalação (testes, `cargo run`), os ganchos não gravam nada (seção 6).
    fn installed() -> bool {
        UpdateManager::new(NoneSource {}, None, None).is_ok()
    }

    fn reg(args: &[&str]) {
        // Um registro que falha não pode travar a instalação: o duplo clique só não funciona.
        let _ = Command::new("reg")
            .args(args)
            .creation_flags(CREATE_NO_WINDOW)
            .status();
    }

    /// Instalar e atualizar: registra o `.lamina` para o executável atual.
    pub fn register() {
        if !installed() {
            return;
        }
        let Ok(exe) = std::env::current_exe() else {
            return;
        };
        for entry in windows_entries(&exe) {
            match &entry.name {
                Some(name) => reg(&["add", &entry.key, "/v", name, "/d", &entry.data, "/f"]),
                None => reg(&["add", &entry.key, "/ve", "/d", &entry.data, "/f"]),
            }
        }
    }

    /// Desinstalar: apaga só as chaves que a instalação criou.
    pub fn unregister() {
        if !installed() {
            return;
        }
        for key in windows_keys_to_remove() {
            reg(&["delete", &key, "/f"]);
        }
    }
}

#[cfg(windows)]
pub use windows::{register, unregister};

/// No Linux, o AppImage não tem instalação: na abertura, grava o atalho, o tipo do arquivo e o
/// ícone na pasta do usuário, se mudaram (o AppImage pode ter sido movido).
#[cfg(target_os = "linux")]
pub fn linux_desktop() {
    use std::path::{Path, PathBuf};

    use lamina_editor::updates::association::{linux_desktop_entry, linux_mime_xml};

    const ICON: &[u8] = include_bytes!("../icons/lamina-512.png");

    let (Some(appimage), Some(home)) = (std::env::var_os("APPIMAGE"), std::env::var_os("HOME"))
    else {
        return;
    };
    let share = std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(home).join(".local").join("share"));
    let write = |path: &Path, bytes: &[u8]| -> bool {
        if std::fs::read(path).is_ok_and(|old| old == bytes) {
            return false;
        }
        path.parent()
            .is_some_and(|dir| std::fs::create_dir_all(dir).is_ok())
            && std::fs::write(path, bytes).is_ok()
    };
    let apps = share.join("applications");
    let mime = share.join("mime");
    let desktop = linux_desktop_entry(Path::new(&appimage));
    let changed_desktop = write(&apps.join("lamina.desktop"), desktop.as_bytes());
    let changed_mime = write(
        &mime.join("packages").join("lamina.xml"),
        linux_mime_xml().as_bytes(),
    );
    write(&share.join("icons/hicolor/512x512/apps/lamina.png"), ICON);
    // As ferramentas que atualizam os bancos podem faltar: sem elas, vale no próximo login.
    if changed_mime {
        let _ = std::process::Command::new("update-mime-database")
            .arg(&mime)
            .status();
    }
    if changed_desktop {
        let _ = std::process::Command::new("update-desktop-database")
            .arg(&apps)
            .status();
    }
}
