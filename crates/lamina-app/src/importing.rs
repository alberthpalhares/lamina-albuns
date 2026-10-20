//! Fotos entrando no álbum (spec IM, parte de importar): "Importar fotos…" (Ctrl+I), "Importar
//! uma pasta de fotos…" e arquivos soltos do Explorador, pelo gancho do winit (ADR 0011, decisão
//! 9). A importação roda fora da linha da janela; as contas são do `lamina-editor`.

use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;
use std::time::Duration;

use lamina_editor::importing::{WAIT_IMPORT, import_photos};
use lamina_editor::{Human, guarded};
use slint::winit_030::{EventResult, WinitWindowAccessor, winit};
use slint::{ComponentHandle, Timer};

use crate::state::{App, Msg};
use crate::{AppWindow, ImportUi};

type Shared = Rc<RefCell<App>>;

/// Os arquivos soltos chegam um evento por arquivo; depois de uma pausa curta, vão juntos.
const GATHER: Duration = Duration::from_millis(150);

/// "Importar fotos…" (vários arquivos) ou "Importar uma pasta de fotos…".
pub fn pick(app: &mut App, folder: bool) {
    let dialog = rfd::FileDialog::new().set_title(if folder {
        "Pasta com as fotos para importar"
    } else {
        "Fotos para importar"
    });
    let paths = if folder {
        dialog.pick_folder().map(|p| vec![p])
    } else {
        dialog
            .add_filter(
                "Fotos",
                &["jpg", "jpeg", "png", "tif", "tiff", "heic", "heif"],
            )
            .pick_files()
    };
    if let Some(paths) = paths {
        start(app, paths);
    }
}

/// Importa `paths` no álbum aberto; sem álbum, abre o Novo projeto com elas (regra 6).
pub fn start(app: &mut App, paths: Vec<PathBuf>) {
    if app.editor.busy() {
        app.notice = WAIT_IMPORT.to_owned();
        return app.refresh_current();
    }
    let Some(session) = app.session() else {
        return crate::new_project::start_with_files(app, paths);
    };
    let file = session.file().map(std::path::Path::to_path_buf);
    if let Some(w) = app.window.upgrade() {
        w.set_busy_text("Importando as fotos…".into());
    }
    app.editor.set_busy(true);
    let (cache, tx) = (app.cache.clone(), app.tx.clone());
    std::thread::spawn(move || {
        let progress = |done: usize, total: usize| {
            // Se a janela já fechou, não há a quem avisar.
            let _ = tx.send(Msg::Importing(done, total));
        };
        let result = guarded(|| import_photos(&paths, &cache, &progress).map_err(|e| e.human()))
            .and_then(|r| r);
        let _ = tx.send(Msg::Imported(result, file));
    });
}

/// O andamento e o fim da importação.
pub fn message(app: &mut App, msg: Msg) {
    let Some(w) = app.window.upgrade() else {
        return;
    };
    match msg {
        Msg::Importing(done, total) => {
            w.set_busy_text(format!("Importando {done} de {total} fotos…").into());
        }
        Msg::Imported(result, file) => {
            w.set_busy_text("".into());
            app.editor.set_busy(false);
            // O álbum trocou no meio (não deveria, com o editor ocupado): nada a fazer.
            if app
                .session()
                .map(|s| s.file().map(std::path::Path::to_path_buf))
                != Some(file)
            {
                return;
            }
            match result.map(|report| app.editor.add_photos(&report)) {
                Ok(Ok(summary)) => {
                    app.project_changed();
                    app.notice = summary.text();
                    app.refresh_current();
                }
                Ok(Err(e)) => crate::files::show_error(app, &e.human()),
                Err(problem) => crate::files::show_error(app, &problem),
            }
        }
        _ => {}
    }
}

/// O gancho do winit: arquivos sobre a janela mostram "Solte para importar"; soltos, vão juntos
/// para a importação depois de uma pausa curta.
pub fn connect(window: &AppWindow, app: &Shared) {
    let pending: Rc<RefCell<Vec<PathBuf>>> = Rc::default();
    let a = app.clone();
    let weak = window.as_weak();
    window.window().on_winit_window_event(move |_, event| {
        let show = |on: bool| {
            if let Some(w) = weak.upgrade() {
                w.global::<ImportUi>().set_dropping(on);
            }
        };
        match event {
            winit::event::WindowEvent::HoveredFile(_) => show(true),
            winit::event::WindowEvent::HoveredFileCancelled => show(false),
            winit::event::WindowEvent::DroppedFile(path) => {
                show(false);
                let first = pending.borrow().is_empty();
                pending.borrow_mut().push(path.clone());
                if first {
                    let (a, pending) = (a.clone(), pending.clone());
                    Timer::single_shot(GATHER, move || {
                        let paths = std::mem::take(&mut *pending.borrow_mut());
                        start(&mut a.borrow_mut(), paths);
                    });
                }
            }
            _ => {}
        }
        EventResult::Propagate
    });
}
