//! As faixas de baixo da janela (spec PN): lâminas, linha do tempo e fotos, cada uma com título e
//! recolhível. Quais estão recolhidas fica em `janela.json`, na pasta de configuração, e vale na
//! próxima vez que o Lamina abrir. A versão do arquivo e a instalação nova são da spec JE.

use std::io::Write as _;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::Session;
use crate::photos::used_photos;

/// Arquivo do estado da janela, ao lado das preferências de layout.
pub const STRIPS_FILE: &str = "janela.json";

/// Aviso quando o arquivo não lê (PN, regra 4; JE, seção 6).
pub const STRIPS_UNREADABLE: &str =
    "Não foi possível ler as preferências da janela; voltaram ao padrão.";

/// A versão do estado da janela (spec FX, regra 11): um arquivo de antes da 3 é lido com a faixa
/// Lâminas aberta, uma vez só (a JE, na versão 2, a desligava).
pub const STRIPS_VERSION: u8 = 3;

/// Quem abre o Lamina (spec JE, regra 2): sem `janela.json`, a instalação nova começa com o
/// painel da direita escondido e quem já usava (tem `ultima_versao` em `atualizacoes.json`), com
/// ele aberto.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Install {
    New,
    Existing,
}

impl Install {
    /// Pela última versão aberta gravada em `atualizacoes.json` (spec DS).
    pub fn from_last_version(last: Option<&str>) -> Install {
        if last.is_some() {
            Install::Existing
        } else {
            Install::New
        }
    }
}

/// Uma das três faixas, de cima para baixo.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Strip {
    Spreads,
    Timeline,
    Photos,
    /// O painel da direita (spec AF, regra 11): Alt+4 dá o espaço para a lâmina.
    Panel,
}

/// Quais faixas estão abertas (`true`) ou recolhidas.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Strips {
    /// A versão do estado da janela (spec JE). Arquivo sem o campo vale 0 (de antes da JE).
    #[serde(rename = "versao", default)]
    pub version: u8,
    #[serde(rename = "laminas")]
    pub spreads: bool,
    #[serde(rename = "linha_do_tempo")]
    pub timeline: bool,
    #[serde(rename = "fotos")]
    pub photos: bool,
    /// O painel da direita aberto (spec AF, regra 11). Arquivo antigo sem o campo vale aberto.
    #[serde(rename = "painel")]
    pub panel: bool,
    /// A unidade da tela e o formato do último álbum criado (spec NP, regras 1 e 3).
    #[serde(rename = "unidade")]
    pub unit: crate::new_project::Unit,
    #[serde(rename = "ultimo_formato")]
    pub format: crate::new_project::Format,
    /// "Reduzir movimento" (spec MV, regra 14): também é estado da janela. Arquivo antigo sem o
    /// campo vale desligado.
    #[serde(rename = "reduzir_movimento")]
    pub reduce_motion: bool,
    /// O zoom das miniaturas do painel de fotos, de 64 a 240 px (spec PF, regra 4). Sem o campo,
    /// 96; fora da faixa, preso a ela ao ler.
    #[serde(rename = "fotos_zoom")]
    pub photos_zoom: u32,
    /// A altura do painel de fotos em px lógicos (spec PF, regra 5); sem o campo, duas linhas.
    #[serde(rename = "fotos_altura", skip_serializing_if = "Option::is_none")]
    pub photos_height: Option<u32>,
    /// As faixas de baixo que o botão das faixas escondeu e vai mostrar de novo (spec FX, regra 8):
    /// lâminas, sequência e fotografias.
    #[serde(rename = "faixas_lembradas", skip_serializing_if = "Option::is_none")]
    pub remembered: Option<[bool; 3]>,
    /// A largura do painel da direita em px lógicos (spec FX, regra 13); sem o campo, 300.
    #[serde(rename = "painel_largura", skip_serializing_if = "Option::is_none")]
    pub panel_width: Option<u32>,
}

/// Tudo aberto: o editor só na memória (testes e linha de comando) e os campos que faltam num
/// arquivo antigo. A instalação nova é [`Strips::fresh`].
impl Default for Strips {
    fn default() -> Self {
        Strips {
            version: STRIPS_VERSION,
            spreads: true,
            timeline: true,
            photos: true,
            panel: true,
            unit: crate::new_project::Unit::default(),
            format: crate::new_project::Format::Square,
            reduce_motion: false,
            photos_zoom: crate::photo_grid::ZOOM_DEFAULT,
            photos_height: None,
            remembered: None,
            panel_width: None,
        }
    }
}

impl Strips {
    /// O padrão de uma instalação nova e de um arquivo ilegível (spec JE, regra 2): o painel
    /// escondido e as três faixas de baixo abertas (spec FX, regra 10).
    pub fn fresh() -> Self {
        Strips {
            panel: false,
            ..Strips::default()
        }
    }

    /// Sem `janela.json`, como fica a janela de quem já usava ou de quem acabou de instalar.
    fn without_file(install: Install) -> Self {
        match install {
            Install::New => Strips::fresh(),
            Install::Existing => Strips::default(),
        }
    }

    /// Um arquivo de antes da versão 3 abre a faixa Lâminas uma vez (spec FX, regra 11); um de
    /// versão maior (um Lamina mais novo) vale como está.
    fn upgraded(mut self) -> Self {
        if self.version < STRIPS_VERSION {
            self.spreads = true;
            self.version = STRIPS_VERSION;
        }
        self
    }

    pub fn is_open(self, strip: Strip) -> bool {
        match strip {
            Strip::Spreads => self.spreads,
            Strip::Timeline => self.timeline,
            Strip::Photos => self.photos,
            Strip::Panel => self.panel,
        }
    }

    /// Recolhe a faixa aberta ou abre a recolhida. Mexer numa faixa de baixo sozinha esquece o que
    /// o botão das faixas lembrava (spec FX, regra 8).
    pub fn toggle(&mut self, strip: Strip) {
        if strip != Strip::Panel {
            self.remembered = None;
        }
        let open = match strip {
            Strip::Spreads => &mut self.spreads,
            Strip::Timeline => &mut self.timeline,
            Strip::Photos => &mut self.photos,
            Strip::Panel => &mut self.panel,
        };
        *open = !*open;
    }

    /// O botão das faixas de baixo (spec FX, regra 8): com alguma aberta, esconde as três e lembra
    /// quais estavam; com as três escondidas, abre as lembradas (sem nada lembrado, as três).
    pub fn toggle_bottom(&mut self) {
        let open = [self.spreads, self.timeline, self.photos];
        let next = if open.contains(&true) {
            self.remembered = Some(open);
            [false; 3]
        } else {
            self.remembered
                .take()
                .filter(|r| r.contains(&true))
                .unwrap_or([true; 3])
        };
        [self.spreads, self.timeline, self.photos] = next;
    }
}

/// O estado das faixas e o arquivo onde ele fica (nenhum = só na memória, nos testes).
#[derive(Debug, Default)]
pub struct StripsStore {
    path: Option<PathBuf>,
    pub strips: Strips,
    /// Aviso de leitura para a janela mostrar uma vez.
    pub warning: Option<String>,
    /// As calibrações do tamanho real, por monitor (spec ZR, regra 8); fora do `Strips`.
    pub(crate) calibrations: Vec<crate::calibration::Calibration>,
}

impl StripsStore {
    /// Lê o arquivo de quem já usava o Lamina ([`Install::Existing`]).
    pub fn load(path: &Path) -> Self {
        StripsStore::load_for(path, Install::Existing)
    }

    /// Lê o arquivo (spec JE, regra 2). Sem arquivo, o padrão de `install`; ilegível, o padrão
    /// novo com aviso. Nada é gravado até a próxima mudança.
    pub fn load_for(path: &Path, install: Install) -> Self {
        let mut store = StripsStore {
            path: Some(path.to_path_buf()),
            strips: Strips::without_file(install),
            warning: None,
            calibrations: Vec::new(),
        };
        let Ok(text) = std::fs::read_to_string(path) else {
            return store;
        };
        match serde_json::from_str::<Strips>(&text) {
            Ok(strips) => {
                store.strips = strips.upgraded();
                store.calibrations = crate::calibration::read_list(&text);
            }
            Err(_) => {
                store.strips = Strips::fresh();
                store.warning = Some(STRIPS_UNREADABLE.to_owned());
            }
        }
        // Zoom fora da faixa é preso a ela (spec PF, regra 4).
        let zoom = i64::from(store.strips.photos_zoom);
        store.strips.photos_zoom = crate::photo_grid::clamp_zoom(zoom);
        store
    }

    /// Grava com segurança (temporário + renomear). `Err` com a mensagem da seção 6.
    pub fn save(&self) -> Result<(), String> {
        let Some(path) = &self.path else {
            return Ok(());
        };
        write(path, &self.strips, &self.calibrations)
            .map_err(|reason| format!("Não foi possível guardar o estado dos painéis: {reason}."))
    }
}

/// O arquivo inteiro: o estado das faixas e as calibrações (que só vão quando há alguma).
#[derive(Serialize)]
struct WindowFile<'a> {
    #[serde(flatten)]
    strips: &'a Strips,
    #[serde(rename = "tamanho_real", skip_serializing_if = "<[_]>::is_empty")]
    calibrations: &'a [crate::calibration::Calibration],
}

fn write(
    path: &Path,
    strips: &Strips,
    calibrations: &[crate::calibration::Calibration],
) -> Result<(), String> {
    let folder = path.parent().unwrap_or(Path::new("."));
    std::fs::create_dir_all(folder).map_err(|e| e.to_string())?;
    let file = WindowFile {
        strips,
        calibrations,
    };
    let json = serde_json::to_string_pretty(&file).map_err(|e| e.to_string())?;
    let mut tmp = tempfile::NamedTempFile::new_in(folder).map_err(|e| e.to_string())?;
    tmp.write_all(json.as_bytes()).map_err(|e| e.to_string())?;
    tmp.persist(path).map_err(|e| e.error.to_string())?;
    Ok(())
}

/// Os títulos das três faixas, com o resumo atual (regra 1): "Lâminas · 4 lâminas (8 páginas)",
/// "Sequência · …" e "Fotografias · 12 de 12 usadas" (nomes da spec FX, regra 12).
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
        "Sequência · arraste um corte para passar fotos entre lâminas".to_owned(),
        photos_title(session, 0),
    ]
}

/// O título do painel de fotos com a seleção (spec PF, regra 12): "Fotografias · 4 de 10 usadas ·
/// 3 selecionadas"; sem seleção, sem a última parte.
pub fn photos_title(session: &Session, selected: usize) -> String {
    let project = session.project();
    let title = format!(
        "Fotografias · {} de {} usadas",
        used_photos(project),
        project.photos.len()
    );
    match selected {
        0 => title,
        1 => format!("{title} · 1 selecionada"),
        n => format!("{title} · {n} selecionadas"),
    }
}
