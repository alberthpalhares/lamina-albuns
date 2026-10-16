//! A janela Exportar (spec EI, regras 1 a 4): opções, pré-impressão com "Ir para a lâmina" e
//! "Ignorar", a pergunta para substituir e a exportação em segundo plano, com andamento e resumo.

use std::path::{Path, PathBuf};

use lamina_editor::{
    DEFAULT_QUALITY, ExportOptions, export_progress, export_project, export_summary,
};

use crate::AppWindow;
use crate::convert::model;
use crate::files::show_error;
use crate::state::{App, Msg};

/// Abre a janela com a pasta da última exportação (ou a padrão) e as opções padrão da EX.
pub fn start(app: &mut App) {
    let (Some(w), Some(session)) = (app.window.upgrade(), app.session()) else {
        return;
    };
    w.set_ex_folder(session.export_folder().display().to_string().into());
    w.set_ex_all(true);
    w.set_ex_range("".into());
    w.set_ex_pages(false);
    w.set_ex_quality(i32::from(DEFAULT_QUALITY));
    w.set_ex_step(0);
    app.export_check = None;
    changed(app);
    w.set_show_export(true);
}

/// As opções como estão na janela.
fn options(w: &AppWindow, overwrite: bool) -> ExportOptions {
    let range = w.get_ex_range();
    ExportOptions {
        quality: u8::try_from(w.get_ex_quality().clamp(1, 100)).unwrap_or(DEFAULT_QUALITY),
        range: (!w.get_ex_all()).then(|| range.trim().to_owned()),
        pages: w.get_ex_pages(),
        overwrite,
    }
}

fn folder(w: &AppWindow) -> PathBuf {
    PathBuf::from(w.get_ex_folder().as_str())
}

/// Confere as opções a cada mudança: o erro aparece e "Exportar" fica desabilitado.
pub fn changed(app: &mut App) {
    let (Some(w), Some(session)) = (app.window.upgrade(), app.session()) else {
        return;
    };
    let error = session
        .check_export(&folder(&w), &options(&w, false))
        .err()
        .map(|e| e.to_string())
        .unwrap_or_default();
    w.set_ex_error(error.into());
}

pub fn pick_folder(app: &mut App) {
    let Some(w) = app.window.upgrade() else {
        return;
    };
    let current = folder(&w);
    let start = if current.is_dir() {
        current
    } else {
        current.parent().map(Path::to_path_buf).unwrap_or_default()
    };
    if let Some(chosen) = rfd::FileDialog::new()
        .set_title("Pasta da exportação")
        .set_directory(start)
        .pick_folder()
    {
        w.set_ex_folder(chosen.display().to_string().into());
        changed(app);
    }
}

/// "Exportar": confere as lâminas escolhidas; com avisos, mostra a pré-impressão (regra 2).
pub fn export(app: &mut App) {
    let (Some(w), Some(session)) = (app.window.upgrade(), app.session()) else {
        return;
    };
    match session.check_export(&folder(&w), &options(&w, false)) {
        Ok(check) => {
            let warned = !check.warnings.is_empty();
            app.export_check = Some(check);
            if warned {
                show_warnings(app);
                w.set_ex_step(1);
            } else {
                proceed(app);
            }
        }
        Err(e) => w.set_ex_error(e.to_string().into()),
    }
}

fn show_warnings(app: &App) {
    let (Some(w), Some(check)) = (app.window.upgrade(), &app.export_check) else {
        return;
    };
    let lines = check
        .warnings
        .iter()
        .map(|x| x.to_string().into())
        .collect();
    w.set_ex_warnings(model(lines));
}

/// "Ir para a lâmina": fecha a janela e mostra a lâmina do aviso, sem exportar.
pub fn go_to(app: &mut App, index: i32) {
    let warning = usize::try_from(index)
        .ok()
        .and_then(|i| app.export_check.as_ref()?.warnings.get(i).cloned());
    let Some(warning) = warning else {
        return;
    };
    if let Some(w) = app.window.upgrade() {
        w.set_show_export(false);
        w.invoke_focus_main();
    }
    if app.editor.go_to_warning(&warning) {
        app.refresh_current();
    }
}

pub fn ignore(app: &mut App, index: i32) {
    let ignored = usize::try_from(index)
        .ok()
        .zip(app.export_check.as_mut())
        .is_some_and(|(i, check)| check.ignore(i));
    if ignored {
        show_warnings(app);
    }
}

/// Depois dos avisos: pergunta antes de substituir arquivos que já existem (regra 4).
pub fn proceed(app: &mut App) {
    let Some(w) = app.window.upgrade() else {
        return;
    };
    let existing = app.export_check.as_ref().map_or(0, |c| c.existing.len());
    if existing == 0 {
        run(app, false);
        return;
    }
    let files = if existing == 1 {
        "1 arquivo desta exportação".to_owned()
    } else {
        format!("{existing} arquivos desta exportação")
    };
    let text = format!(
        "Já existem {files} em {}. Substituindo, os arquivos lamina-NN.jpg e pagina-NN.jpg que \
         não fizerem parte desta exportação também saem da pasta; os outros arquivos ficam.",
        folder(&w).display()
    );
    w.set_ex_replace_text(text.into());
    w.set_ex_step(2);
}

pub fn back(app: &mut App) {
    if let Some(w) = app.window.upgrade() {
        w.set_ex_step(0);
    }
}

/// Exporta fora da linha da janela, que fica parada com o andamento até terminar (regra 3).
pub fn run(app: &mut App, overwrite: bool) {
    let (Some(w), Some(session)) = (app.window.upgrade(), app.editor.session_mut()) else {
        return;
    };
    let (folder, options) = (folder(&w), options(&w, overwrite));
    session.set_export_folder(&folder);
    let project = session.project().clone();
    w.set_show_export(false);
    w.set_busy_text("Exportando…".into());
    let tx = app.tx.clone();
    std::thread::spawn(move || {
        let progress = |done: usize, total: usize| {
            // Se a janela já fechou, não há a quem avisar.
            let _ = tx.send(Msg::Exporting(done, total));
        };
        let result = export_project(&project, &folder, &options, &progress)
            .map(|report| export_summary(&report, &folder))
            .map_err(|e| e.to_string());
        let _ = tx.send(Msg::Exported(result, folder));
    });
}

/// Andamento e fim da exportação.
pub fn message(app: &mut App, msg: Msg) {
    let Some(w) = app.window.upgrade() else {
        return;
    };
    match msg {
        Msg::Exporting(done, total) => w.set_busy_text(export_progress(done, total).into()),
        Msg::Exported(result, folder) => {
            w.set_busy_text("".into());
            match result {
                Ok(summary) => {
                    w.set_message_title("Exportação concluída".into());
                    w.set_message_text(summary.into());
                    w.set_message_folder(folder.display().to_string().into());
                    w.set_show_message(true);
                }
                Err(e) => show_error(app, "Não foi possível exportar", &e),
            }
        }
        _ => {}
    }
}

/// "Abrir a pasta": o gerenciador de arquivos do sistema.
pub fn open_folder(app: &App) {
    let Some(w) = app.window.upgrade() else {
        return;
    };
    let folder = w.get_message_folder().to_string();
    let opener = if cfg!(windows) {
        "explorer"
    } else if cfg!(target_os = "macos") {
        "open"
    } else {
        "xdg-open"
    };
    if let Err(e) = std::process::Command::new(opener).arg(&folder).spawn() {
        show_error(app, "Não foi possível abrir a pasta", &e.to_string());
    }
}
