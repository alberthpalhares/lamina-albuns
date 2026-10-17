//! A janela "Novo álbum" (spec JP, seção 3): escolhe a pasta, mostra a sugestão, confere os
//! campos a cada mudança e monta o álbum em segundo plano, com andamento.

use lamina_editor::forms::{SpecText, album_name_error};
use lamina_editor::{
    NewAlbum, SpecFields, Step, check_spreads, count_photos, create_album, folder_name,
    suggestion_for, suggestion_text,
};
use lamina_layout::Reuse;

use crate::AppWindow;
use crate::files::show_error;
use crate::state::{App, Msg};

/// Escolhe a pasta e abre a janela com os padrões do `lamina build`.
pub fn start(app: &mut App) {
    let Some(folder) = rfd::FileDialog::new()
        .set_title("Pasta das fotos do álbum")
        .pick_folder()
    else {
        return;
    };
    let photos = match count_photos(&folder) {
        Ok(0) => {
            let message = format!("Nenhuma foto encontrada em {}.", folder.display());
            show_error(app, "Novo álbum", &message);
            return;
        }
        Ok(n) => n,
        Err(e) => return show_error(app, "Novo álbum", &e.to_string()),
    };
    let Some(w) = app.window.upgrade() else {
        return;
    };
    let d = SpecFields::default();
    let text = SpecText::from(&d);
    let spec = d.to_spec().ok();
    let suggested = spec.map_or(1, |s| suggestion_for(photos, &s));
    w.set_na_folder(folder.display().to_string().into());
    w.set_na_name(folder_name(&folder).into());
    w.set_na_width(text.width_cm.into());
    w.set_na_height(text.height_cm.into());
    w.set_na_dpi(text.dpi.into());
    w.set_na_bleed(text.bleed_mm.into());
    w.set_na_safe(text.safe_mm.into());
    w.set_na_spine(text.spine_mm.into());
    w.set_na_starts(text.starts_single);
    w.set_na_ends(text.ends_single);
    w.set_na_spreads(i32::try_from(suggested).unwrap_or(1));
    w.set_na_reuse(1);
    w.set_na_photos(i32::try_from(photos).unwrap_or(i32::MAX));
    app.new_folder = folder;
    app.new_photos = photos;
    changed(app);
    w.set_show_new(true);
}

/// Os campos da janela como medidas; `Err` com a mensagem do campo errado.
fn fields(w: &AppWindow) -> Result<SpecFields, String> {
    SpecText {
        width_cm: w.get_na_width().into(),
        height_cm: w.get_na_height().into(),
        dpi: w.get_na_dpi().into(),
        bleed_mm: w.get_na_bleed().into(),
        safe_mm: w.get_na_safe().into(),
        spine_mm: w.get_na_spine().into(),
        starts_single: w.get_na_starts(),
        ends_single: w.get_na_ends(),
    }
    .parse()
}

/// Confere tudo a cada mudança: o erro aparece na janela e "Criar" fica desabilitado.
pub fn changed(app: &mut App) {
    let Some(w) = app.window.upgrade() else {
        return;
    };
    let photos = app.new_photos;
    let spreads = usize::try_from(w.get_na_spreads()).unwrap_or(0);
    let check = fields(&w)
        .and_then(|f| f.to_spec().map_err(|e| e.to_string()))
        .and_then(|spec| {
            w.set_na_suggestion(suggestion_text(suggestion_for(photos, &spec)).into());
            check_spreads(photos, spreads, &spec).map_err(|e| e.to_string())
        });
    let name_error = album_name_error(&w.get_na_name()).map(str::to_owned);
    let error = check.err().or(name_error).unwrap_or_default();
    w.set_na_error(error.into());
}

/// "Criar": monta o álbum fora da linha da janela (regra 4).
pub fn create(app: &mut App) {
    let Some(w) = app.window.upgrade() else {
        return;
    };
    let spec = match fields(&w).and_then(|f| f.to_spec().map_err(|e| e.to_string())) {
        Ok(spec) => spec,
        Err(e) => return w.set_na_error(e.into()),
    };
    let new = NewAlbum {
        folder: app.new_folder.clone(),
        name: Some(w.get_na_name().trim().to_owned()),
        spec,
        spreads: usize::try_from(w.get_na_spreads()).ok(),
        reuse: match w.get_na_reuse() {
            0 => Reuse::Low,
            2 => Reuse::High,
            _ => Reuse::Medium,
        },
        preferences: app.editor.preferences().clone(),
    };
    w.set_show_new(false);
    w.set_busy_text("Importando as fotos…".into());
    app.editor.set_busy(true);
    let (cache, tx) = (app.cache.clone(), app.tx.clone());
    std::thread::spawn(move || {
        let progress = |step: Step| {
            // Se a janela já fechou, não há a quem avisar.
            let _ = tx.send(Msg::Step(step));
        };
        let result = create_album(&new, &cache, &progress)
            .map(|(built, failures)| (built.project, failures.len()))
            .map_err(|e| e.to_string());
        let _ = tx.send(Msg::Album(result));
    });
}

/// Mensagens do trabalho em segundo plano.
pub fn message(app: &mut App, msg: Msg) {
    let Some(w) = app.window.upgrade() else {
        return;
    };
    match msg {
        Msg::Step(Step::Importing { done, total }) => {
            w.set_busy_text(format!("Importando {done} de {total} fotos…").into());
        }
        Msg::Step(Step::Building) => w.set_busy_text("Montando o álbum…".into()),
        Msg::Album(result) => {
            w.set_busy_text("".into());
            app.editor.set_busy(false);
            w.invoke_focus_main();
            match result {
                Ok((project, failures)) => {
                    app.editor.adopt(project);
                    app.project_changed();
                    if failures > 0 {
                        app.notice = format!("{failures} foto(s) não puderam ser importadas.");
                        app.refresh_current();
                    }
                }
                Err(e) => show_error(app, "Não foi possível criar o álbum", &e),
            }
        }
        Msg::ImagesReady => app.images_ready(),
        Msg::Original(id, image) => crate::crop::original_ready(app, id, image),
        Msg::Exporting(..) | Msg::Exported(..) => crate::export::message(app, msg),
        Msg::Searching(..) | Msg::Relinked { .. } => crate::relink::message(app, msg),
    }
}
