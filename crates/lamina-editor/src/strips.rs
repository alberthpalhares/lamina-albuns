//! As faixas de baixo da janela (spec PN): lâminas, linha do tempo e fotos, cada uma com título e
//! recolhível. Quais estão recolhidas fica em `janela.json`, na pasta de configuração, e vale na
//! próxima vez que o Lâmina abrir.

use std::io::Write as _;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::Session;
use crate::photos::used_photos;

/// Arquivo do estado da janela, ao lado das preferências de layout.
pub const STRIPS_FILE: &str = "janela.json";

/// Aviso quando o arquivo não lê (regra 4).
pub const STRIPS_UNREADABLE: &str =
    "Não foi possível ler o estado dos painéis; todos foram abertos.";

/// Uma das três faixas, de cima para baixo.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Strip {
    Spreads,
    Timeline,
    Photos,
}

/// Quais faixas estão abertas (`true`) ou recolhidas.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Strips {
    #[serde(rename = "laminas")]
    pub spreads: bool,
    #[serde(rename = "linha_do_tempo")]
    pub timeline: bool,
    #[serde(rename = "fotos")]
    pub photos: bool,
}

impl Default for Strips {
    fn default() -> Self {
        Strips {
            spreads: true,
            timeline: true,
            photos: true,
        }
    }
}

impl Strips {
    pub fn is_open(self, strip: Strip) -> bool {
        match strip {
            Strip::Spreads => self.spreads,
            Strip::Timeline => self.timeline,
            Strip::Photos => self.photos,
        }
    }

    /// Recolhe a faixa aberta ou abre a recolhida.
    pub fn toggle(&mut self, strip: Strip) {
        let open = match strip {
            Strip::Spreads => &mut self.spreads,
            Strip::Timeline => &mut self.timeline,
            Strip::Photos => &mut self.photos,
        };
        *open = !*open;
    }
}

/// O estado das faixas e o arquivo onde ele fica (nenhum = só na memória, nos testes).
#[derive(Debug, Default)]
pub struct StripsStore {
    path: Option<PathBuf>,
    pub strips: Strips,
    /// Aviso de leitura para a janela mostrar uma vez.
    pub warning: Option<String>,
}

impl StripsStore {
    /// Lê o arquivo. Sem arquivo, todas abertas; ilegível, todas abertas com aviso.
    pub fn load(path: &Path) -> Self {
        let mut store = StripsStore {
            path: Some(path.to_path_buf()),
            ..StripsStore::default()
        };
        let Ok(text) = std::fs::read_to_string(path) else {
            return store;
        };
        match serde_json::from_str(&text) {
            Ok(strips) => store.strips = strips,
            Err(_) => store.warning = Some(STRIPS_UNREADABLE.to_owned()),
        }
        store
    }

    /// Grava com segurança (temporário + renomear). `Err` com a mensagem da seção 6.
    pub fn save(&self) -> Result<(), String> {
        let Some(path) = &self.path else {
            return Ok(());
        };
        write(path, &self.strips)
            .map_err(|reason| format!("Não foi possível guardar o estado dos painéis: {reason}."))
    }
}

fn write(path: &Path, strips: &Strips) -> Result<(), String> {
    let folder = path.parent().unwrap_or(Path::new("."));
    std::fs::create_dir_all(folder).map_err(|e| e.to_string())?;
    let json = serde_json::to_string_pretty(strips).map_err(|e| e.to_string())?;
    let mut tmp = tempfile::NamedTempFile::new_in(folder).map_err(|e| e.to_string())?;
    tmp.write_all(json.as_bytes()).map_err(|e| e.to_string())?;
    tmp.persist(path).map_err(|e| e.error.to_string())?;
    Ok(())
}

/// Os títulos das três faixas, com o resumo atual (regra 1): "Lâminas · 4 lâminas (8 páginas)",
/// "Linha do tempo · …" e "Fotos · 12 de 12 usadas".
pub fn strip_titles(session: &Session) -> [String; 3] {
    let project = session.project();
    let spreads = project.spreads.len();
    let pages = project.spec.edges().page_count(spreads);
    let plural = |n: usize, word: &str| {
        if n == 1 {
            format!("{n} {word}")
        } else {
            format!("{n} {word}s")
        }
    };
    [
        format!(
            "Lâminas · {} ({})",
            plural(spreads, "lâmina"),
            plural(pages, "página")
        ),
        "Linha do tempo · arraste um corte para passar fotos entre lâminas".to_owned(),
        format!(
            "Fotos · {} de {} usadas",
            used_photos(project),
            project.photos.len()
        ),
    ]
}
