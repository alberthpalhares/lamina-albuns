//! Os arquivos que o sistema entrega ao Lamina (spec MA): o duplo clique, o "Abrir com", o ícone
//! do Dock e o `lamina-app <arquivo>`. A decisão é do `lamina_editor::received`; aqui só executa.
//!
//! No Windows e no Linux o arquivo chega como argumento. No Mac chega pelo evento "abrir
//! documentos", que o `fmv-macos-events` recebe (ADR 0013); um temporizador esvazia a caixa dele
//! quando o estado está livre. O evento "sair" do Mac segue o caminho de fechar a janela.

use std::cell::{Cell, RefCell};
use std::path::{Path, PathBuf};
use std::time::Duration;

use lamina_editor::received::Received;
use slint::Timer;

use crate::Shared;
use crate::files;
use crate::state::{App, Pending};

thread_local! {
    /// O álbum recebido que espera a resposta de "Salvar as alterações?".
    static WAITING: RefCell<Option<PathBuf>> = const { RefCell::new(None) };
    /// O relatório de `--verificar-abertura`, se é essa a chamada.
    static REPORT: RefCell<Option<PathBuf>> = const { RefCell::new(None) };
    /// O relatório já foi escrito (o tempo limite não escreve por cima).
    static REPORTED: Cell<bool> = const { Cell::new(false) };
}

/// Quanto a verificação escondida espera por um arquivo (regra 10).
const WAIT_FOR: Duration = Duration::from_secs(30);

/// `lamina-app --verificar-abertura <relatório> [álbum]` (regra 10): o relatório e, fora do Mac,
/// o álbum que faz as vezes do duplo clique.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Check {
    pub report: PathBuf,
    pub file: Option<PathBuf>,
}

/// A verificação escondida, se é essa a chamada. Sem o relatório, sai com 1 sem janela.
pub fn requested() -> Option<Check> {
    let mut args = std::env::args_os().skip(1);
    if args.next()? != "--verificar-abertura" {
        return None;
    }
    let Some(report) = args.next() else {
        eprintln!("--verificar-abertura: falta o arquivo do relatório");
        std::process::exit(1);
    };
    Some(Check {
        report: PathBuf::from(report),
        file: args.next().map(PathBuf::from),
    })
}

/// O argumento da linha de comando (Windows, Linux e a volta depois de atualizar). O `-psn_` que
/// o Mac antigo passava não é arquivo.
pub fn from_args(app: &mut App) {
    if let Some(file) = std::env::args_os()
        .nth(1)
        .filter(|a| !a.to_string_lossy().starts_with("-psn_"))
    {
        receive(app, &[PathBuf::from(file)]);
    }
}

/// Executa a decisão do editor para os arquivos recebidos (regras 2 a 4).
pub fn receive(app: &mut App, paths: &[PathBuf]) {
    match app.editor.receive(paths) {
        Received::Open { path, ask, notice } => {
            if ask {
                WAITING.set(Some(path));
                files::request(app, Pending::OpenReceived);
            } else {
                open(app, &path);
            }
            if let Some(notice) = notice {
                app.notice = notice;
                app.refresh_current();
            }
        }
        Received::AlreadyOpen => {
            if let Some(file) = app.session().and_then(|s| s.file()).map(Path::to_path_buf) {
                report(app, &file);
            }
        }
        Received::Ignore { notice } => {
            fail(&format!("erro: {notice}"));
            app.notice = notice;
            app.refresh_current();
        }
    }
}

/// "Salvar as alterações?" respondida com Salvar ou Não salvar: abre o álbum que esperava.
pub fn proceed(app: &mut App) {
    if let Some(path) = WAITING.take() {
        open(app, &path);
    }
}

fn open(app: &mut App, path: &Path) {
    files::open_file(app, path);
    report(app, path);
}

/// Na verificação escondida: o álbum abriu? Grava o relatório e fecha depois de mostrar.
fn report(app: &App, path: &Path) {
    if REPORT.with_borrow(Option::is_none) {
        return;
    }
    let Some(session) = app.session().filter(|s| s.file() == Some(path)) else {
        fail("erro: o álbum não abriu");
        return;
    };
    let count = session.spread_count();
    write(&format!("aberto: {} ({count} lâminas)", path.display()));
    Timer::single_shot(crate::verify::SHOW_FOR, || {
        // Fechando: um erro aqui só deixa a janela aberta até o tempo limite do CI.
        let _ = slint::quit_event_loop();
    });
}

/// Na verificação escondida, se nada foi relatado ainda: grava o motivo e sai com 1.
fn fail(line: &str) {
    if REPORT.with_borrow(Option::is_some) && !REPORTED.get() {
        write(line);
        std::process::exit(1);
    }
}

fn write(line: &str) {
    REPORTED.set(true);
    REPORT.with_borrow(|report| {
        if let Some(report) = report
            && let Err(e) = std::fs::write(report, format!("{line}\n"))
        {
            eprintln!("--verificar-abertura: {}: {e}", report.display());
        }
    });
}

/// O que precisa viver até a janela fechar: o ouvinte do Mac e o temporizador dele.
#[derive(Debug)]
pub struct Listener {
    #[cfg(target_os = "macos")]
    _mac: mac::Mac,
}

/// Começa a ouvir os arquivos do sistema; com a verificação escondida, marca o tempo limite e,
/// fora do Mac, entrega o álbum do argumento.
pub fn listen(app: &Shared, check: Option<Check>) -> Listener {
    if let Some(check) = check {
        REPORT.set(Some(check.report));
        Timer::single_shot(WAIT_FOR, || fail("erro: nenhum arquivo chegou"));
        if let Some(file) = check.file {
            receive(&mut app.borrow_mut(), &[file]);
        }
    }
    Listener {
        #[cfg(target_os = "macos")]
        _mac: mac::Mac::listen(app),
    }
}

#[cfg(target_os = "macos")]
mod mac {
    use fmv_macos_events::{Event, Inbox, Registration};
    use slint::{Timer, TimerMode};

    use crate::Shared;
    use crate::state::Pending;

    /// De quanto em quanto tempo a caixa do Mac é esvaziada.
    const STEP: std::time::Duration = std::time::Duration::from_millis(100);

    pub struct Mac {
        _registration: Registration,
        _timer: Timer,
    }

    impl std::fmt::Debug for Mac {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.write_str("Mac")
        }
    }

    impl Mac {
        /// Registra os tratadores antes de o laço de eventos começar (ADR 0013).
        pub fn listen(app: &Shared) -> Mac {
            let (registration, inbox) = Registration::install();
            let timer = Timer::default();
            let app = app.clone();
            timer.start(TimerMode::Repeated, STEP, move || drain(&app, &inbox));
            Mac {
                _registration: registration,
                _timer: timer,
            }
        }
    }

    fn drain(app: &Shared, inbox: &Inbox) {
        // Com um diálogo de arquivo aberto, o estado está emprestado: o arquivo espera.
        let Ok(mut app) = app.try_borrow_mut() else {
            return;
        };
        for event in inbox.drain() {
            match event {
                Event::Open(paths) => super::receive(&mut app, &paths),
                Event::Quit => crate::files::request(&mut app, Pending::Close),
            }
        }
    }
}
