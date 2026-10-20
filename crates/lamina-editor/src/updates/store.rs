//! As preferências de atualização em `atualizacoes.json`, na pasta de configuração (spec DS,
//! regra 9): checar ao abrir, receber as versões de teste e a última versão que abriu.

use std::io::Write as _;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// Arquivo das preferências de atualização, ao lado das outras.
pub const UPDATES_FILE: &str = "atualizacoes.json";

/// Aviso quando o arquivo não lê (regra 9).
pub const UPDATES_UNREADABLE: &str =
    "Não foi possível ler as preferências de atualização; voltaram ao padrão.";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct UpdatePrefs {
    /// Checar ao abrir (ligado por padrão).
    #[serde(rename = "automatico")]
    pub automatic: bool,
    /// Receber as versões de teste (desligado por padrão; decisão 4).
    pub beta: bool,
    /// A versão da última abertura, para o aviso "Lamina atualizado para …" (regra 6).
    #[serde(rename = "ultima_versao")]
    pub last_version: Option<String>,
    /// O fotógrafo já ligou ou desligou as versões de teste no menu: a escolha dele vale
    /// (auditoria da Fase 4, achado 8).
    #[serde(rename = "beta_escolhido")]
    pub beta_chosen: bool,
}

impl Default for UpdatePrefs {
    fn default() -> Self {
        UpdatePrefs {
            automatic: true,
            beta: false,
            last_version: None,
            beta_chosen: false,
        }
    }
}

/// As preferências e o arquivo onde elas ficam (nenhum = só na memória, nos testes).
#[derive(Debug, Default)]
pub struct UpdatesStore {
    path: Option<PathBuf>,
    pub prefs: UpdatePrefs,
    /// Aviso de leitura para a janela mostrar uma vez.
    pub warning: Option<String>,
    /// `false` depois de um arquivo ilegível: só volta a gravar quando uma preferência muda
    /// (DS-B11), para não apagar o que a pessoa talvez queira consertar à mão.
    unreadable: bool,
}

impl UpdatesStore {
    /// Lê o arquivo. Sem arquivo, o padrão; ilegível, o padrão com aviso.
    pub fn load(path: &Path) -> Self {
        let mut store = UpdatesStore {
            path: Some(path.to_path_buf()),
            ..UpdatesStore::default()
        };
        let Ok(text) = std::fs::read_to_string(path) else {
            return store;
        };
        match serde_json::from_str(&text) {
            Ok(prefs) => store.prefs = prefs,
            Err(_) => {
                store.warning = Some(UPDATES_UNREADABLE.to_owned());
                store.unreadable = true;
            }
        }
        store
    }

    /// Só na memória (testes e o app sem pasta de configuração).
    pub fn in_memory(prefs: UpdatePrefs) -> Self {
        UpdatesStore {
            prefs,
            ..UpdatesStore::default()
        }
    }

    /// Liga ou desliga a checagem ao abrir e grava.
    pub fn set_automatic(&mut self, on: bool) -> Result<(), String> {
        self.prefs.automatic = on;
        self.unreadable = false;
        self.save()
    }

    /// Liga ou desliga as versões de teste e grava.
    pub fn set_beta(&mut self, on: bool) -> Result<(), String> {
        self.prefs.beta = on;
        self.prefs.beta_chosen = true;
        self.unreadable = false;
        self.save()
    }

    /// Liga as versões de teste sem marcar como escolha do fotógrafo (versão de teste instalada).
    pub fn beta_by_default(&mut self) -> Result<(), String> {
        self.prefs.beta = true;
        if self.unreadable {
            return Ok(());
        }
        self.save()
    }

    /// Guarda a versão desta abertura (não grava sobre um arquivo ilegível).
    pub fn remember_version(&mut self, version: &str) -> Result<(), String> {
        self.prefs.last_version = Some(version.to_owned());
        if self.unreadable {
            return Ok(());
        }
        self.save()
    }

    fn save(&self) -> Result<(), String> {
        let Some(path) = &self.path else {
            return Ok(());
        };
        write(path, &self.prefs).map_err(|reason| {
            format!("Não foi possível guardar as preferências de atualização: {reason}.")
        })
    }
}

fn write(path: &Path, prefs: &UpdatePrefs) -> Result<(), String> {
    let folder = path.parent().unwrap_or(Path::new("."));
    std::fs::create_dir_all(folder).map_err(|e| e.to_string())?;
    let json = serde_json::to_string_pretty(prefs).map_err(|e| e.to_string())?;
    let mut tmp = tempfile::NamedTempFile::new_in(folder).map_err(|e| e.to_string())?;
    tmp.write_all(json.as_bytes()).map_err(|e| e.to_string())?;
    tmp.persist(path).map_err(|e| e.error.to_string())?;
    Ok(())
}
