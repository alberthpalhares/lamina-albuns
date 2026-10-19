//! O registro de erros (spec SR, regras 5 a 9 e 12): um diário em texto, na pasta de
//! configuração, com cada erro que a janela mostrou e cada falha inesperada. Só sai da máquina se
//! o fotógrafo anexar o arquivo no formulário.

use std::fs::{self, OpenOptions};
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use lamina_core::CaptureTime;

use crate::human::HumanMessage;

/// A pasta do registro, dentro da pasta de configuração.
pub const LOG_FOLDER: &str = "registros";
pub const LOG_FILE: &str = "lamina.log";
/// O arquivo anterior, quando o atual passa do limite (regra 7).
pub const OLD_LOG_FILE: &str = "lamina-anterior.log";
/// 1 MB: passando disso, o arquivo vira o anterior.
pub const LOG_LIMIT: u64 = 1024 * 1024;

#[derive(Debug, Clone)]
pub struct ErrorLog {
    folder: PathBuf,
    /// A pasta pessoal, trocada por `~` em todas as linhas (regra 6).
    home: Option<String>,
}

impl ErrorLog {
    /// O registro em `folder`, com a pasta pessoal do sistema.
    pub fn new(folder: PathBuf) -> Self {
        let home =
            std::env::var_os(if cfg!(windows) { "USERPROFILE" } else { "HOME" }).map(PathBuf::from);
        ErrorLog::with_home(folder, home.as_deref())
    }

    /// O registro com outra pasta pessoal (testes).
    pub fn with_home(folder: PathBuf, home: Option<&Path>) -> Self {
        let home = home
            .map(|h| h.to_string_lossy().trim_end_matches(['/', '\\']).to_owned())
            .filter(|h| !h.is_empty());
        ErrorLog { folder, home }
    }

    pub fn folder(&self) -> &Path {
        &self.folder
    }

    /// A pasta, criada se faltar, para abrir no explorador (regra 10).
    pub fn ready_folder(&self) -> std::io::Result<PathBuf> {
        fs::create_dir_all(&self.folder)?;
        Ok(self.folder.clone())
    }

    /// A linha de cada abertura: versão, sistema e canal (regra 5).
    pub fn start(&self, version: &str, system: &str, channel: &str) {
        self.write(&format!(
            "início · Lamina {version} · {system} · canal {channel}"
        ));
    }

    /// Um erro mostrado na janela, com o detalhe indentado embaixo (regra 5).
    pub fn error(&self, message: &HumanMessage) {
        let mut entry = format!("erro · {} · {}", message.title, message.what_to_do);
        for line in message.details.lines() {
            entry.push_str("\n    detalhe: ");
            entry.push_str(line);
        }
        self.write(&entry);
    }

    /// Uma falha inesperada (regra 8): a mensagem do pânico e o lugar no código.
    pub fn unexpected(&self, message: &str, place: &str) {
        self.write(&format!("falha inesperada · {message} · {place}"));
    }

    /// Acrescenta uma entrada com a data. Qualquer falha é ignorada (regra 9).
    fn write(&self, entry: &str) {
        let text = format!("{} UTC · {entry}\n", now_utc());
        let text = match &self.home {
            Some(home) => text.replace(home.as_str(), "~"),
            None => text,
        };
        let _ = self.append(&text);
    }

    fn append(&self, text: &str) -> std::io::Result<()> {
        fs::create_dir_all(&self.folder)?;
        let file = self.folder.join(LOG_FILE);
        if fs::metadata(&file).is_ok_and(|m| m.len() > LOG_LIMIT) {
            // Se a troca falhar (arquivo aberto em outro programa), segue no mesmo arquivo.
            let _ = fs::rename(&file, self.folder.join(OLD_LOG_FILE));
        }
        let mut out = OpenOptions::new().create(true).append(true).open(file)?;
        out.write_all(text.as_bytes())
    }
}

/// A data e a hora de agora, em UTC (regra 12: o registro não é saída do projeto).
fn now_utc() -> String {
    let seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| i64::try_from(d.as_secs()).unwrap_or(0));
    CaptureTime::from_seconds(seconds).to_string()
}

/// Liga o gancho de pânico: grava a falha no registro e segue para o gancho que já existia
/// (regra 8). O gancho só tenta gravar; se falhar, não faz mais nada.
pub fn install_panic_hook(log: ErrorLog) {
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let place = info
            .location()
            .map_or_else(String::new, |l| format!("{}:{}", l.file(), l.line()));
        log.unexpected(&crate::human::panic_text(info.payload()), &place);
        previous(info);
    }));
}
