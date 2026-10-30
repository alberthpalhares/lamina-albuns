//! A janela "Algumas fotos mudaram de lugar" (spec RF): as pastas ausentes, "Mostrar onde
//! está…", "Tentar de novo" e "Localizar esta foto…". A procura e a conferência correm fora da
//! linha da janela; os textos e as decisões vêm do editor.

use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use lamina_core::PhotoId;
use lamina_editor::relink_text::{
    DIFFERENT, HINT, INTRO, PLUG_IN, TITLE, not_found_in, relink_view,
};
use lamina_editor::{
    Discovery, Found, Human, HumanMessage, Located, MissingFolder, discover, guarded, locate,
    missing_among, photo_paths, relinked_message, search_moved,
};
use slint::{ComponentHandle, ModelRc, Timer, TimerMode, VecModel};

use crate::state::{App, Msg};
use crate::{AppWindow, RelinkRowUi, RelinkUi};

type Shared = Rc<RefCell<App>>;

thread_local! {
    // As pastas mostradas (para saber qual "Mostrar onde está…" foi clicado), a pasta da procura
    // em curso e o arquivo de outro conteúdo esperando o "Usar mesmo assim".
    static SHOWN: RefCell<Vec<MissingFolder>> = const { RefCell::new(Vec::new()) };
    static SEARCHING: RefCell<Option<MissingFolder>> = const { RefCell::new(None) };
    static WAITING: RefCell<Option<Found>> = const { RefCell::new(None) };
    // O que a descoberta achou (regra 11) e se uma conferência já está rodando (regra 10).
    static DISCOVERED: RefCell<Vec<Discovery>> = const { RefCell::new(Vec::new()) };
    static CHECKING: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    static EVERY: Timer = Timer::default();
}

/// De quanto em quanto tempo o Lamina confere se as fotos continuam no lugar (regra 10).
const EVERY_SECONDS: u64 = 5;

/// Abre (ou refaz) a janela com o que falta agora; sem nada faltando, ela fecha.
pub fn show(app: &App) {
    let Some(w) = app.window.upgrade() else {
        return;
    };
    let ui = w.global::<RelinkUi>();
    let Some(view) = app.session().and_then(relink_view) else {
        ui.set_show(false);
        return;
    };
    let discovered = DISCOVERED.with_borrow(|d| d.clone());
    let solved = |f: &MissingFolder| discovered.iter().any(|d| d.folder().path == f.path);
    let rows: Vec<RelinkRowUi> = view
        .folders
        .iter()
        .filter(|f| !solved(f))
        .map(|f| RelinkRowUi {
            name: f.name().into(),
            detail: f.place().into(),
            unplugged: f.disk.is_some(),
        })
        .collect();
    SHOWN.set(view.folders.into_iter().filter(|f| !solved(f)).collect());
    let sentences: Vec<String> = discovered.iter().map(Discovery::sentence).collect();
    ui.set_discovered(
        sentences
            .join(
                "
",
            )
            .into(),
    );
    ui.set_title(TITLE.into());
    ui.set_intro(INTRO.into());
    ui.set_hint(HINT.into());
    ui.set_plug_in(if view.unplugged { PLUG_IN } else { "" }.into());
    ui.set_more(view.more.unwrap_or_default().into());
    ui.set_rows(ModelRc::from(Rc::new(VecModel::from(rows))));
    ui.set_question("".into());
    ui.set_show(true);
}

/// Depois da procura de sempre ao abrir: a janela abre uma vez, se ainda faltam fotos (regra 2).
pub fn after_open(app: &mut App) {
    if app
        .editor
        .session_mut()
        .is_some_and(|s| s.take_relink_prompt())
    {
        show(app);
    }
}

fn is_open(app: &App) -> bool {
    app.window
        .upgrade()
        .is_some_and(|w| w.global::<RelinkUi>().get_show())
}

/// Uma procura que o fotógrafo pediu terminou: a janela mostra o que ainda falta, ou o aviso de
/// que nada foi achado (seção 6). `true` = a janela cuidou do resultado.
pub fn searched(app: &App, changed: usize, folder: &Path) -> bool {
    if !is_open(app) {
        return false;
    }
    let pending = SEARCHING.take();
    show(app);
    if changed == 0
        && let (Some(w), Some(f)) = (app.window.upgrade(), pending)
    {
        w.global::<RelinkUi>()
            .set_status(not_found_in(&f, folder).into());
    }
    true
}

/// "Mostrar onde está…" da pasta `index`: a pasta nova, e a procura em segundo plano (regra 3).
fn pick(app: &mut App, index: usize) {
    let Some(folder) = SHOWN.with_borrow(|f| f.get(index).cloned()) else {
        return;
    };
    let title = format!("Onde está a pasta “{}” agora?", folder.name());
    let answer = crate::dialogs::dialog(app, &title).pick_folder();
    crate::dialogs::then(answer, move |app, picked| {
        if let Some(to) = picked {
            search(app, folder, to.path().to_path_buf());
        }
    });
}

fn search(app: &mut App, from: MissingFolder, to: PathBuf) {
    let Some(session) = app.session() else {
        return;
    };
    let (project, missing) = (session.project().clone(), session.missing().to_vec());
    let file = session.file().map(Path::to_path_buf);
    if let Some(w) = app.window.upgrade() {
        w.global::<RelinkUi>().set_status("".into());
        w.set_busy_text("Procurando as fotos…".into());
    }
    app.editor.set_busy(true);
    let old = from.path.clone();
    SEARCHING.set(Some(from));
    let tx = app.tx.clone();
    std::thread::spawn(move || {
        let progress = |done: usize, total: usize| {
            let _ = tx.send(Msg::Searching(done, total));
        };
        let found = guarded(|| search_moved(&project, &missing, Some(&old), &to, &progress));
        let _ = tx.send(match found {
            Ok(found) => Msg::Relinked {
                file,
                found,
                folder: Some(to),
            },
            Err(problem) => Msg::Failed(problem),
        });
    });
}

/// A conferência de 5 em 5 segundos, de "Tentar de novo" e da volta para a janela: os arquivos
/// continuam no lugar? Só olha se existem, fora da linha da janela; se a anterior não terminou,
/// esta é pulada (regras 6 e 10). Não é uma edição.
pub fn recheck(app: &App) {
    let Some(session) = app.session() else {
        return;
    };
    if CHECKING.get() {
        return;
    }
    CHECKING.set(true);
    let paths = photo_paths(session.project());
    let file = session.file().map(Path::to_path_buf);
    let tx = app.tx.clone();
    std::thread::spawn(move || {
        let still = missing_among(&paths);
        let _ = tx.send(Msg::Rechecked { file, still });
    });
}

pub fn rechecked(app: &mut App, file: Option<PathBuf>, still: &[PhotoId]) {
    CHECKING.set(false);
    let Some(session) = app.editor.session_mut() else {
        return;
    };
    if session.file().map(Path::to_path_buf) != file || !session.checked_missing(still) {
        return;
    }
    let ask = session.take_relink_prompt();
    app.project_changed();
    if session_missing(app) == 0 {
        app.notice.clear();
        DISCOVERED.set(Vec::new());
    }
    app.refresh_current();
    if ask {
        find_out(app);
    } else if is_open(app) {
        show(app);
    }
}

fn session_missing(app: &App) -> usize {
    app.session().map_or(0, |s| s.missing().len())
}

/// Fotos sumiram com o álbum aberto: descobre o que aconteceu, em segundo plano, e abre a
/// janela com isso (regras 11 e 12).
fn find_out(app: &App) {
    let Some(session) = app.session() else {
        return;
    };
    let (project, missing) = (session.project().clone(), session.missing().to_vec());
    let file = session.file().map(Path::to_path_buf);
    let tx = app.tx.clone();
    std::thread::spawn(move || {
        let found = guarded(|| discover(&project, &missing, file.as_deref())).unwrap_or_default();
        let _ = tx.send(Msg::Discovered { file, found });
    });
}

pub fn discovered(app: &mut App, file: Option<PathBuf>, found: Vec<Discovery>) {
    if app.session().and_then(|s| s.file().map(Path::to_path_buf)) != file {
        return;
    }
    DISCOVERED.set(found);
    show(app);
}

/// "Usar o lugar novo": tudo o que a descoberta achou, numa edição só (regra 11).
fn use_new(app: &mut App) {
    let found: Vec<Found> =
        DISCOVERED.with_borrow(|d| d.iter().flat_map(|x| x.found().to_vec()).collect());
    DISCOVERED.set(Vec::new());
    apply(app, &found);
}

/// Liga a conferência de 5 em 5 segundos (regra 10).
fn start_checking(app: &Shared) {
    let a = Rc::downgrade(app);
    EVERY.with(|t| {
        t.start(
            TimerMode::Repeated,
            std::time::Duration::from_secs(EVERY_SECONDS),
            move || {
                if let Some(app) = a.upgrade()
                    && let Ok(app) = app.try_borrow()
                {
                    recheck(&app);
                }
            },
        );
    });
}

/// "Localizar esta foto…" (regra 4): o arquivo, e a conferência em segundo plano.
pub fn locate_photo(app: &mut App, id: PhotoId) {
    let answer = crate::dialogs::dialog(app, "Localizar esta foto").pick_file();
    crate::dialogs::then(answer, move |app, picked| {
        let (Some(picked), Some(session)) = (picked, app.session()) else {
            return;
        };
        let (project, missing) = (session.project().clone(), session.missing().to_vec());
        let file = session.file().map(Path::to_path_buf);
        let path = picked.path().to_path_buf();
        let tx = app.tx.clone();
        std::thread::spawn(move || {
            let result = guarded(|| locate(&project, &missing, id, &path))
                .and_then(|r| r.map_err(|e| e.human()));
            let _ = tx.send(Msg::Located { file, result });
        });
    });
}

pub fn located(app: &mut App, file: Option<PathBuf>, result: Result<Located, HumanMessage>) {
    if app.session().and_then(|s| s.file().map(Path::to_path_buf)) != file {
        return;
    }
    match result {
        Ok(Located::Same(found)) => apply(app, &found),
        Ok(Located::Different(found)) => {
            WAITING.set(Some(found));
            if let Some(w) = app.window.upgrade() {
                let ui = w.global::<RelinkUi>();
                ui.set_question(DIFFERENT.into());
                ui.set_show(true);
            }
        }
        Err(problem) => crate::files::show_error(app, &problem),
    }
}

/// As fotos achadas entram como uma edição só (regra 7), e a janela mostra o que falta.
fn apply(app: &mut App, found: &[Found]) {
    let Some(session) = app.editor.session_mut() else {
        return;
    };
    let changed = session.relink(found);
    let still = session.missing().len();
    if changed > 0 {
        app.project_changed();
        app.notice = relinked_message(changed, still).unwrap_or_default();
        app.refresh_current();
    }
    if is_open(app) {
        show(app);
    }
}

pub fn connect(window: &AppWindow, app: &Shared) {
    let ui = window.global::<RelinkUi>();
    let a = app.clone();
    ui.on_pick(move |i| {
        if let Ok(i) = usize::try_from(i) {
            pick(&mut a.borrow_mut(), i);
        }
    });
    let a = app.clone();
    ui.on_retry(move || recheck(&a.borrow()));
    let a = app.clone();
    ui.on_use_new(move || use_new(&mut a.borrow_mut()));
    start_checking(app);
    ui.on_later(|| SEARCHING.set(None));
    let a = app.clone();
    let w = window.as_weak();
    ui.on_answer(move |yes| {
        let waiting = WAITING.take();
        let mut app = a.borrow_mut();
        if let Some(w) = w.upgrade() {
            w.global::<RelinkUi>().set_question("".into());
            w.global::<RelinkUi>().set_show(false);
        }
        if let (true, Some(found)) = (yes, waiting) {
            apply(&mut app, std::slice::from_ref(&found));
        }
    });
}
