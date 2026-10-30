//! Presets (spec PR): especificações prontas, com a faixa de páginas e o que a encadernadora pede na
//! exportação. Os do Lamina vêm do programa (tamanhos comuns e, desde 2026-10-29, os que uma
//! encadernadora nos passou, em `lab_presets`); os do fotógrafo ficam em `presets.json`, na pasta de
//! configuração.

use std::io::Write as _;
use std::path::{Path, PathBuf};

use lamina_core::{ExportPrefs, PageLimits};
use serde::{Deserialize, Serialize};

use crate::album::SpecFields;
use crate::human::Human as _;

/// Arquivo dos presets do fotógrafo, ao lado das preferências de layout.
pub const PRESETS_FILE: &str = "presets.json";
/// Versão do formato do arquivo: a 3 guarda o laboratório (spec ET, regra 8); a 2, a sangria e a
/// zona segura por borda (spec NP); a 1, lida como margens iguais nas bordas, não tinha esses campos.
const VERSION: u32 = 3;

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
    /// A encadernadora do preset, só um rótulo para filtrar (spec ET); sem ela, um tamanho comum.
    /// No disco continua `laboratorio`, para a beta 6 ler o arquivo (correção de 2026-10-29).
    #[serde(
        rename = "laboratorio",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub binder: Option<String>,
    /// O tipo de álbum da encadernadora, quando ela tem tipos com regras diferentes. Previsto
    /// nos dados, ainda sem tela (spec ET, correção de 2026-10-29).
    #[serde(rename = "tipo", default, skip_serializing_if = "Option::is_none")]
    pub album_type: Option<String>,
}

/// Os presets do Lamina (regra 1): os tamanhos padrão de mercado de cada formato, largura ×
/// altura da página (decisão do dono, 2026-10-19), com o nome igual à medida ("30x30cm"), todos a 300 DPI, 3 mm de sangria, 10 mm de
/// zona segura, 20 a 80 páginas, JPEG em sRGB. Depois deles, os de laboratório.
pub fn builtin_presets() -> Vec<Preset> {
    let mut all = common_presets();
    all.extend(crate::lab_presets::proalbuns_presets());
    all
}

fn common_presets() -> Vec<Preset> {
    [
        (15.0, 15.0),
        (20.0, 20.0),
        (25.0, 25.0),
        (30.0, 30.0),
        (21.0, 15.0),
        (25.0, 20.0),
        (30.0, 20.0),
        (30.0, 25.0),
        (35.0, 30.0),
        (40.0, 30.0),
        (45.0, 30.0),
        (15.0, 21.0),
        (20.0, 25.0),
        (20.0, 30.0),
        (25.0, 30.0),
    ]
    .into_iter()
    .map(|(width_cm, height_cm)| Preset {
        name: crate::preset_names::size_name(width_cm, height_cm),
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
        binder: None,
        album_type: None,
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
    #[error("Já existe um preset chamado \"{0}\".")]
    Exists(String),
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

    /// O preset de nome `name`, do Lamina ou do fotógrafo: pelo nome completo, "25x20cm
    /// (ProAlbuns)", ou pelo nome sozinho (PR-B13).
    pub fn find(&self, name: &str) -> Option<Preset> {
        crate::preset_names::find_preset(&self.all(), name).cloned()
    }

    /// Já existe um preset do fotógrafo com esse nome na encadernadora (para perguntar antes de
    /// substituir)? O mesmo nome em encadernadoras diferentes são dois presets (spec ET, seção 6).
    pub fn exists(&self, name: &str, binder: Option<&str>) -> bool {
        let binder = crate::new_project::clean_binder(binder);
        self.user
            .iter()
            .any(|p| p.name == name.trim() && p.binder == binder)
    }

    /// Salva (ou substitui) o preset e grava o arquivo (regra 2).
    pub fn save(&mut self, mut preset: Preset) -> Result<(), PresetError> {
        preset.name = preset.name.trim().to_owned();
        preset.binder = crate::new_project::clean_binder(preset.binder.as_deref());
        check(&preset)?;
        match self
            .user
            .iter_mut()
            .find(|p| p.name == preset.name && p.binder == preset.binder)
        {
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
        if self.exists(&preset.name, preset.binder.as_deref()) && preset.name != old {
            return Err(PresetError::Exists(preset.name));
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
    // O mesmo nome de um preset do Lamina só vale com outra encadernadora (como os da ProAlbuns).
    if builtin_presets()
        .iter()
        .any(|p| p.name == preset.name && p.binder == preset.binder)
    {
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
        .map_err(|e| PresetError::Spec(e.human().line()))
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
