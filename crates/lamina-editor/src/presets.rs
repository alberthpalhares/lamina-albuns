//! Presets (spec PR): especificações prontas, com a faixa de páginas e o que o laboratório pede na
//! exportação. Os do Lamina vêm do programa (tamanhos comuns, sem nome de laboratório: regra 6 do
//! AGENTS); os do fotógrafo ficam em `presets.json`, na pasta de configuração.

use std::io::Write as _;
use std::path::{Path, PathBuf};

use lamina_core::{ExportPrefs, PageLimits};
use serde::{Deserialize, Serialize};

use crate::album::SpecFields;

/// Arquivo dos presets do fotógrafo, ao lado das preferências de layout.
pub const PRESETS_FILE: &str = "presets.json";
/// Versão do formato do arquivo.
const VERSION: u32 = 1;

/// Uma especificação pronta.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Preset {
    #[serde(rename = "nome")]
    pub name: String,
    #[serde(rename = "especificacao")]
    pub fields: SpecFields,
    #[serde(rename = "paginas", default)]
    pub limits: PageLimits,
    #[serde(rename = "exportacao", default)]
    pub export: ExportPrefs,
}

/// Os seis presets do Lamina (regra 1): 300 DPI, 3 mm de sangria, 10 mm de zona segura, 20 a 80
/// páginas, JPEG em sRGB.
pub fn builtin_presets() -> Vec<Preset> {
    [
        ("Quadrado 20 × 20", 20.0, 20.0),
        ("Quadrado 25 × 25", 25.0, 25.0),
        ("Quadrado 30 × 30", 30.0, 30.0),
        ("Paisagem 30 × 20", 30.0, 20.0),
        ("Paisagem 40 × 30", 40.0, 30.0),
        ("Retrato 20 × 30", 20.0, 30.0),
    ]
    .into_iter()
    .map(|(name, width_cm, height_cm)| Preset {
        name: name.to_owned(),
        fields: SpecFields {
            width_cm,
            height_cm,
            ..SpecFields::default()
        },
        limits: PageLimits {
            min: Some(20),
            max: Some(80),
        },
        export: ExportPrefs::default(),
    })
    .collect()
}

/// Por que um preset não pôde ser salvo (seção 6).
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum PresetError {
    #[error("Dê um nome ao preset.")]
    EmptyName,
    #[error("Esse nome é de um preset do Lamina; escolha outro.")]
    Builtin,
    #[error("O mínimo de páginas passa do máximo.")]
    LimitsOrder,
    #[error("{0}")]
    Spec(String),
    #[error("Não foi possível gravar os presets: {0}.")]
    Write(String),
}

/// No disco: a versão e os presets do fotógrafo.
#[derive(Serialize, Deserialize)]
struct OnDisk {
    versao: u32,
    presets: Vec<Preset>,
}

/// Os presets do fotógrafo e o arquivo deles (nenhum = só na memória, nos testes).
#[derive(Debug, Default)]
pub struct PresetsStore {
    path: Option<PathBuf>,
    user: Vec<Preset>,
    /// Aviso de leitura para a janela mostrar uma vez.
    pub warning: Option<String>,
}

impl PresetsStore {
    /// Lê o arquivo. Sem arquivo, nenhum preset do fotógrafo; ilegível, nenhum, com aviso.
    pub fn load(path: &Path) -> Self {
        let mut store = PresetsStore {
            path: Some(path.to_path_buf()),
            ..PresetsStore::default()
        };
        let Ok(text) = std::fs::read_to_string(path) else {
            return store;
        };
        match serde_json::from_str::<OnDisk>(&text) {
            Ok(disk) if disk.versao <= VERSION => store.user = disk.presets,
            Ok(disk) => {
                store.warning = Some(format!(
                    "Não foi possível ler os presets: versão {} do arquivo.",
                    disk.versao
                ));
            }
            Err(e) => store.warning = Some(format!("Não foi possível ler os presets: {e}.")),
        }
        store
    }

    /// Os do Lamina e depois os do fotógrafo, na ordem em que foram salvos.
    pub fn all(&self) -> Vec<Preset> {
        let mut all = builtin_presets();
        all.extend(self.user.iter().cloned());
        all
    }

    pub fn user(&self) -> &[Preset] {
        &self.user
    }

    /// O preset de nome `name`, do Lamina ou do fotógrafo.
    pub fn find(&self, name: &str) -> Option<Preset> {
        self.all().into_iter().find(|p| p.name == name)
    }

    /// Já existe um preset do fotógrafo com esse nome (para perguntar antes de substituir)?
    pub fn exists(&self, name: &str) -> bool {
        self.user.iter().any(|p| p.name == name.trim())
    }

    /// Salva (ou substitui) o preset e grava o arquivo (regra 2).
    pub fn save(&mut self, mut preset: Preset) -> Result<(), PresetError> {
        preset.name = preset.name.trim().to_owned();
        check(&preset)?;
        match self.user.iter_mut().find(|p| p.name == preset.name) {
            Some(existing) => *existing = preset,
            None => self.user.push(preset),
        }
        self.write()
    }

    /// Apaga o preset do fotógrafo `name`; `false` se não existe.
    pub fn remove(&mut self, name: &str) -> Result<bool, PresetError> {
        let before = self.user.len();
        self.user.retain(|p| p.name != name);
        if self.user.len() == before {
            return Ok(false);
        }
        self.write().map(|()| true)
    }

    /// Troca o nome do preset do fotógrafo `old` por `new`.
    pub fn rename(&mut self, old: &str, new: &str) -> Result<bool, PresetError> {
        let Some(mut preset) = self.user.iter().find(|p| p.name == old).cloned() else {
            return Ok(false);
        };
        preset.name = new.trim().to_owned();
        check(&preset)?;
        if self.exists(&preset.name) && preset.name != old {
            return Err(PresetError::Spec(format!(
                "Já existe um preset chamado \"{}\".",
                preset.name
            )));
        }
        // Renomear deixa o preset no mesmo lugar da lista (auditoria do Marco 3, D5).
        if let Some(slot) = self.user.iter_mut().find(|p| p.name == old) {
            *slot = preset;
        }
        self.write().map(|()| true)
    }

    fn write(&self) -> Result<(), PresetError> {
        let Some(path) = &self.path else {
            return Ok(());
        };
        let disk = OnDisk {
            versao: VERSION,
            presets: self.user.clone(),
        };
        write(path, &disk).map_err(PresetError::Write)
    }
}

/// As regras de um preset salvo: nome, faixa de páginas e especificação válida.
fn check(preset: &Preset) -> Result<(), PresetError> {
    if preset.name.is_empty() {
        return Err(PresetError::EmptyName);
    }
    if builtin_presets().iter().any(|p| p.name == preset.name) {
        return Err(PresetError::Builtin);
    }
    if let (Some(min), Some(max)) = (preset.limits.min, preset.limits.max)
        && min > max
    {
        return Err(PresetError::LimitsOrder);
    }
    preset
        .fields
        .to_spec()
        .map(|_| ())
        .map_err(|e| PresetError::Spec(e.to_string()))
}

fn write(path: &Path, disk: &OnDisk) -> Result<(), String> {
    let folder = path.parent().unwrap_or(Path::new("."));
    std::fs::create_dir_all(folder).map_err(|e| e.to_string())?;
    let json = serde_json::to_string_pretty(disk).map_err(|e| e.to_string())?;
    let mut tmp = tempfile::NamedTempFile::new_in(folder).map_err(|e| e.to_string())?;
    tmp.write_all(json.as_bytes()).map_err(|e| e.to_string())?;
    tmp.persist(path).map_err(|e| e.error.to_string())?;
    Ok(())
}
