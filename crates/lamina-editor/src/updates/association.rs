//! A associação do arquivo `.lamina` com o Lamina (spec DS, regra 10): o que registrar em cada
//! sistema, calculado aqui sem tocar em nada; o `lamina-app` só executa.

use std::path::Path;

/// O tipo do arquivo no Windows e o tipo MIME no Linux.
pub const PROG_ID: &str = "Lamina.Album";
pub const MIME_TYPE: &str = "application/x-lamina";
const DESCRIPTION: &str = "Álbum do Lamina";

/// Um valor do registro do Windows, por usuário. `name: None` é o valor padrão da chave.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegValue {
    pub key: String,
    pub name: Option<String>,
    pub data: String,
}

const CLASSES: &str = r"HKCU\Software\Classes";

/// O que a instalação e a atualização gravam no Windows, com `exe` o caminho do Lamina
/// instalado: a extensão aponta para o tipo, que tem nome, ícone e o comando de abrir.
pub fn windows_entries(exe: &Path) -> Vec<RegValue> {
    let exe = exe.display();
    let value = |key: String, data: String| RegValue {
        key,
        name: None,
        data,
    };
    vec![
        value(format!(r"{CLASSES}\.lamina"), PROG_ID.to_owned()),
        value(format!(r"{CLASSES}\{PROG_ID}"), DESCRIPTION.to_owned()),
        value(
            format!(r"{CLASSES}\{PROG_ID}\DefaultIcon"),
            format!("\"{exe}\",0"),
        ),
        value(
            format!(r"{CLASSES}\{PROG_ID}\shell\open\command"),
            format!("\"{exe}\" \"%1\""),
        ),
    ]
}

/// As chaves que a desinstalação apaga (com tudo o que está dentro): só as que a instalação
/// criou.
pub fn windows_keys_to_remove() -> Vec<String> {
    vec![
        format!(r"{CLASSES}\.lamina"),
        format!(r"{CLASSES}\{PROG_ID}"),
    ]
}

/// O atalho do Linux (`~/.local/share/applications/lamina.desktop`) para o AppImage em
/// `appimage`, com o tipo do arquivo.
pub fn linux_desktop_entry(appimage: &Path) -> String {
    format!(
        "[Desktop Entry]\nType=Application\nName=Lamina\nComment=Diagramação de álbuns de fotos\n\
         Exec=\"{}\" %f\nIcon=lamina\nTerminal=false\nCategories=Graphics;Photography;\n\
         MimeType={MIME_TYPE};\n",
        appimage.display()
    )
}

/// O tipo do arquivo para o banco de tipos do Linux
/// (`~/.local/share/mime/packages/lamina.xml`).
pub fn linux_mime_xml() -> String {
    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
         <mime-info xmlns=\"http://www.freedesktop.org/standards/shared-mime-info\">\n  \
         <mime-type type=\"{MIME_TYPE}\">\n    <comment>{DESCRIPTION}</comment>\n    \
         <glob pattern=\"*.lamina\"/>\n  </mime-type>\n</mime-info>\n"
    )
}
