//! `lamina-app --verificar <álbum>` (spec DS, regra 12): o teste de fumaça da release. Abre a
//! janela, carrega o álbum, desenha a primeira lâmina e fecha: código 0, ou 1 com o motivo.

use std::path::PathBuf;

use lamina_editor::{EXAMPLE_DIR, EXAMPLE_FILE, Editor, EditorError, Human};

/// Tempo com a janela aberta, para a primeira lâmina chegar à tela.
pub const SHOW_FOR: std::time::Duration = std::time::Duration::from_secs(3);

/// O álbum de `--verificar <álbum>`, se é essa a chamada. `--verificar exemplo` é o álbum de
/// exemplo que veio com o programa (spec PC, regra 8).
pub fn requested() -> Option<PathBuf> {
    let mut args = std::env::args_os().skip(1);
    if args.next()? != "--verificar" {
        return None;
    }
    let album = args.next()?;
    Some(if album == EXAMPLE_DIR {
        crate::welcome::example_source().join(EXAMPLE_FILE)
    } else {
        PathBuf::from(album)
    })
}

/// A pasta de `--verificar-importacao <pasta>` (spec TR, regra 9), se é essa a chamada.
pub fn import_requested() -> Option<PathBuf> {
    let mut args = std::env::args_os().skip(1);
    if args.next()? != "--verificar-importacao" {
        return None;
    }
    args.next().map(PathBuf::from)
}

/// Confere a pasta antes de abrir a janela: uma pasta que não existe sai com 1 sem janela.
pub fn check_import(folder: &std::path::Path) -> Result<(), String> {
    if folder.is_dir() {
        Ok(())
    } else {
        Err(format!("a pasta {} não existe", folder.display()))
    }
}

/// O teste de fumaça da importação: um álbum sem fotos e sem lâminas, como "Começar sem fotos",
/// recebe a pasta pelo mesmo caminho de "Importar uma pasta de fotos…".
pub fn start_import(app: &mut crate::state::App, folder: PathBuf) -> Result<(), String> {
    let cm30 = lamina_core::Mm::from_cm(30.0);
    let line = |e: lamina_core::CoreError| e.human().line();
    let dpi = lamina_core::Dpi::new(300).map_err(line)?;
    let spec = lamina_core::AlbumSpec::new(cm30, cm30, dpi).map_err(line)?;
    app.editor
        .adopt(lamina_core::Project::new("Verificação da importação", spec));
    app.project_changed();
    app.verify_import = Some(std::time::Instant::now());
    crate::importing::start(app, vec![folder], None);
    Ok(())
}

/// A importação do teste de fumaça terminou: o resumo vai para a saída de erro e a janela fecha
/// depois de mostrar as fotos.
pub fn import_done(app: &crate::state::App, summary: &str) {
    let seconds = app
        .verify_import
        .map_or(0.0, |start| start.elapsed().as_secs_f64());
    let photos = app.session().map_or(0, |s| s.project().photos.len());
    eprintln!("--verificar-importacao: {photos} fotos no álbum em {seconds:.1} s ({summary})");
    crate::mailbox::until(WAIT_STEP, thumbs_shown);
}

/// De quanto em quanto tempo o teste de fumaça confere se as miniaturas chegaram.
const WAIT_STEP: std::time::Duration = std::time::Duration::from_millis(100);

/// Todas as miniaturas pedidas chegaram à janela: ela fica aberta mais um pouco e fecha.
fn thumbs_shown(app: &mut crate::state::App) -> bool {
    if app.images.thumbs_pending() > 0 {
        return false;
    }
    let seconds = app
        .verify_import
        .map_or(0.0, |start| start.elapsed().as_secs_f64());
    eprintln!("--verificar-importacao: miniaturas na janela em {seconds:.1} s");
    slint::Timer::single_shot(SHOW_FOR, || {
        // Fechando: um erro aqui só deixa a janela aberta até o tempo limite do CI.
        let _ = slint::quit_event_loop();
    });
    true
}

/// A importação do teste de fumaça falhou: sai com 1 e o motivo.
pub fn import_failed(problem: &lamina_editor::HumanMessage) -> ! {
    eprintln!(
        "--verificar-importacao: {}\n{}",
        problem.line(),
        problem.details
    );
    std::process::exit(1)
}

/// Confere o álbum antes de abrir a janela: um arquivo que não é álbum sai com 1 sem janela.
pub fn check(path: &std::path::Path) -> Result<(), String> {
    // O exemplo que não veio com o programa tem a frase própria (spec PC, PC-B8).
    if !path.is_file() && path.file_name().is_some_and(|n| n == EXAMPLE_FILE) {
        let folder = path.parent().unwrap_or(path).display().to_string();
        return Err(EditorError::NoExample(folder).human().line());
    }
    let mut editor = Editor::new();
    editor.open(path).map_err(|e| {
        let message = e.human();
        format!("{}\n{}", message.line(), message.details)
    })?;
    if editor.session().is_some_and(|s| s.spread_count() > 0) {
        Ok(())
    } else {
        Err("o álbum não tem lâminas".to_owned())
    }
}
