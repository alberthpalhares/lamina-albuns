//! Atualização automática (spec DS; ADR 0009): quando checar, o que mostrar e o que pedir ao
//! instalador. A rede e o instalador ficam atrás de [`Updater`], que o `lamina-app` implementa
//! com o Velopack; aqui tudo roda sem rede, com um `Updater` de mentira nos testes.
//!
//! O ciclo: ao abrir, [`Updates::on_open`] pede uma checagem ([`Job::Check`]); o app a roda
//! numa linha separada com [`run`] e devolve o [`Outcome`] em [`Updates::finish`], que pede o
//! download ([`Job::Download`]). Com tudo baixado, a barra aparece ([`Updates::bar`]).

pub mod association;
mod store;
mod version;

use std::ffi::OsString;

pub use store::{UPDATES_FILE, UPDATES_UNREADABLE, UpdatePrefs, UpdatesStore};
pub use version::Version;

use crate::Editor;

/// Os avisos da spec (seção 4).
pub const UP_TO_DATE: &str = "Você está com a versão mais nova";
pub const CHECK_FAILED: &str =
    "Não foi possível verificar agora. Confira a internet e tente de novo.";
pub const NOT_INSTALLED: &str = "As atualizações automáticas funcionam no Lamina instalado.";

/// De onde vêm as versões: as estáveis ou as de teste (regra 7).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Channel {
    Stable,
    Beta,
}

/// Por que o instalador não respondeu.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UpdateError {
    /// O Lamina não está numa instalação do Velopack (`cargo run`, regra 8).
    NotInstalled,
    /// Rede, servidor ou pacote que não confere: a mensagem fica só no registro.
    Failed(String),
}

/// O instalador: o Velopack no app, um de mentira nos testes.
pub trait Updater: Send {
    /// A versão mais nova publicada no canal, se houver alguma.
    fn latest(&mut self, channel: Channel) -> Result<Option<Version>, UpdateError>;
    /// Baixa e confere a versão achada por [`Updater::latest`] no canal.
    fn download(&mut self, channel: Channel, version: &Version) -> Result<(), UpdateError>;
    /// Aplica o que foi baixado e reabre o Lamina com `args`. No app, não volta.
    fn apply_and_restart(&mut self, args: &[OsString]) -> Result<(), UpdateError>;
    /// Aplica o que foi baixado quando o Lamina sair (regra 6), sem reabrir.
    fn apply_on_exit(&mut self) -> Result<(), UpdateError>;
}

/// O trabalho que o app roda fora da linha da janela.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Job {
    /// Procura versão nova; `manual` = pedido pelo menu (regra 3).
    Check {
        manual: bool,
        beta: bool,
    },
    Download {
        channel: Channel,
        version: Version,
    },
}

/// A resposta de um [`Job`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    Found {
        manual: bool,
        channel: Channel,
        version: Version,
    },
    UpToDate {
        manual: bool,
    },
    CheckFailed {
        manual: bool,
        error: UpdateError,
    },
    Downloaded(Version),
    DownloadFailed(Version),
}

/// Roda um trabalho no instalador (na linha separada do app).
pub fn run(updater: &mut dyn Updater, job: &Job, current: &Version) -> Outcome {
    match job {
        Job::Check { manual, beta } => {
            let manual = *manual;
            let mut channels = vec![Channel::Stable];
            if *beta {
                channels.push(Channel::Beta);
            }
            let mut best: Option<(Channel, Version)> = None;
            for channel in channels {
                match updater.latest(channel) {
                    Ok(Some(v)) if v > *current && best.as_ref().is_none_or(|(_, b)| v > *b) => {
                        best = Some((channel, v));
                    }
                    Ok(_) => {}
                    Err(error) => return Outcome::CheckFailed { manual, error },
                }
            }
            match best {
                Some((channel, version)) => Outcome::Found {
                    manual,
                    channel,
                    version,
                },
                None => Outcome::UpToDate { manual },
            }
        }
        Job::Download { channel, version } => match updater.download(*channel, version) {
            Ok(()) => Outcome::Downloaded(version.clone()),
            Err(_) => Outcome::DownloadFailed(version.clone()),
        },
    }
}

/// Em que ponto está a atualização desta abertura.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Phase {
    Idle,
    Checking,
    Downloading(Version),
    /// Baixada e conferida: a barra aparece.
    Ready(Version),
    /// Baixada, mas "Agora não": entra ao fechar.
    Dismissed(Version),
    /// "Reiniciar e atualizar" já pedido.
    Applying(Version),
}

/// O que o app faz depois de uma resposta: um aviso e o próximo trabalho.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Step {
    pub notice: Option<String>,
    pub job: Option<Job>,
}

/// A atualização da abertura atual e as preferências.
#[derive(Debug)]
pub struct Updates {
    pub store: UpdatesStore,
    current: Version,
    installed: bool,
    phase: Phase,
    /// A checagem automática já rodou nesta abertura (regra 2).
    checked: bool,
}

impl Updates {
    /// `current` é a versão que está rodando; `installed`, se veio de um instalador.
    pub fn new(store: UpdatesStore, current: Version, installed: bool) -> Self {
        Updates {
            store,
            current,
            installed,
            phase: Phase::Idle,
            checked: false,
        }
    }

    pub fn current(&self) -> &Version {
        &self.current
    }

    pub fn phase(&self) -> &Phase {
        &self.phase
    }

    /// Ao abrir a janela: o aviso de atualizado (DS-B9), o de preferências ilegíveis (DS-B11) e
    /// a checagem automática, uma vez só (DS-B1, B2 e B12).
    pub fn on_open(&mut self) -> Step {
        let mut notices = Vec::new();
        if let Some(warning) = self.store.warning.take() {
            notices.push(warning);
        }
        // Uma versão de teste instalada recebe as próximas versões de teste, até o fotógrafo
        // escolher no menu (auditoria da Fase 4, achado 8).
        let prefs = &self.store.prefs;
        if self.installed
            && self.current.is_prerelease()
            && !prefs.beta
            && !prefs.beta_chosen
            && let Err(e) = self.store.beta_by_default()
        {
            notices.push(e);
        }
        let current = self.current.to_string();
        let last = self.store.prefs.last_version.clone();
        if last.as_deref() != Some(current.as_str()) {
            let newer = last
                .as_deref()
                .and_then(Version::parse)
                .is_some_and(|l| l < self.current);
            if newer {
                notices.push(format!("Lamina atualizado para {current}."));
            }
            if let Err(e) = self.store.remember_version(&current) {
                notices.push(e);
            }
        }
        let job = if self.installed && self.store.prefs.automatic && !self.checked {
            self.checked = true;
            self.start_check(false)
        } else {
            None
        };
        Step {
            notice: (!notices.is_empty()).then(|| notices.join(" ")),
            job,
        }
    }

    /// "Verificar atualizações agora" (regra 2): checa mesmo com a preferência desligada.
    pub fn check_now(&mut self) -> Step {
        if !self.installed {
            return Step {
                notice: Some(NOT_INSTALLED.to_owned()),
                job: None,
            };
        }
        if let Phase::Dismissed(v) = &self.phase {
            // Já baixada: a barra volta.
            self.phase = Phase::Ready(v.clone());
            return Step::default();
        }
        Step {
            notice: None,
            job: self.start_check(true),
        }
    }

    fn start_check(&mut self, manual: bool) -> Option<Job> {
        if self.phase != Phase::Idle {
            return None;
        }
        self.phase = Phase::Checking;
        Some(Job::Check {
            manual,
            beta: self.store.prefs.beta,
        })
    }

    /// A resposta de um trabalho (regras 3 e 4).
    pub fn finish(&mut self, outcome: Outcome) -> Step {
        match outcome {
            Outcome::Found {
                channel, version, ..
            } => {
                self.phase = Phase::Downloading(version.clone());
                Step {
                    notice: None,
                    job: Some(Job::Download { channel, version }),
                }
            }
            Outcome::UpToDate { manual } => {
                self.phase = Phase::Idle;
                Step {
                    notice: manual.then(|| format!("{UP_TO_DATE} ({}).", self.current)),
                    job: None,
                }
            }
            Outcome::CheckFailed { manual, error } => {
                self.phase = Phase::Idle;
                let notice = match error {
                    _ if !manual => None,
                    UpdateError::NotInstalled => Some(NOT_INSTALLED.to_owned()),
                    UpdateError::Failed(_) => Some(CHECK_FAILED.to_owned()),
                };
                Step { notice, job: None }
            }
            Outcome::Downloaded(version) => {
                self.phase = Phase::Ready(version);
                Step::default()
            }
            Outcome::DownloadFailed(_) => {
                self.phase = Phase::Idle;
                Step::default()
            }
        }
    }

    /// O texto da barra, quando há uma versão pronta (DS-B4).
    pub fn bar(&self) -> Option<String> {
        match &self.phase {
            Phase::Ready(v) => Some(format!("Lamina {v} está pronta.")),
            _ => None,
        }
    }

    /// "Agora não": a barra some nesta abertura (regra 4).
    pub fn dismiss(&mut self) {
        if let Phase::Ready(v) = &self.phase {
            self.phase = Phase::Dismissed(v.clone());
        }
    }

    /// Começa o "Reiniciar e atualizar". `false` se não há versão pronta ou já começou (o
    /// segundo clique é ignorado).
    pub fn begin_restart(&mut self) -> bool {
        match &self.phase {
            Phase::Ready(v) | Phase::Dismissed(v) => {
                self.phase = Phase::Applying(v.clone());
                true
            }
            _ => false,
        }
    }

    /// Volta à barra quando o reinício não seguiu (Cancelar, erro ao salvar).
    pub fn cancel_restart(&mut self) {
        if let Phase::Applying(v) = &self.phase {
            self.phase = Phase::Ready(v.clone());
        }
    }

    /// Ao fechar com uma versão baixada e não aplicada, ela entra depois que o Lamina sair
    /// (regra 6, DS-B8).
    pub fn applies_on_exit(&self) -> bool {
        matches!(
            self.phase,
            Phase::Ready(_) | Phase::Dismissed(_) | Phase::Applying(_)
        )
    }

    /// Liga ou desliga a checagem ao abrir (menu Ajuda).
    pub fn toggle_automatic(&mut self) -> Result<(), String> {
        let on = !self.store.prefs.automatic;
        self.store.set_automatic(on)
    }

    /// Liga ou desliga as versões de teste (menu Ajuda).
    pub fn toggle_beta(&mut self) -> Result<(), String> {
        let on = !self.store.prefs.beta;
        self.store.set_beta(on)
    }
}

/// Os argumentos para reabrir depois de atualizar: o álbum aberto, se tem arquivo (regra 5).
pub fn restart_args(editor: &Editor) -> Vec<OsString> {
    editor
        .session()
        .and_then(|s| s.file())
        .map(|f| vec![f.as_os_str().to_owned()])
        .unwrap_or_default()
}
