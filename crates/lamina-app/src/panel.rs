//! O painel de layouts (spec TL, seção 4): copia o painel do editor para a tela, com uma
//! miniatura de cada opção feita com as fotos de verdade.

use lamina_core::PhotoId;
use lamina_editor::motion::panel_key;
use lamina_editor::text::ALL_REJECTED;
use lamina_editor::{Command, Part, option_title};

use crate::LayoutItem;
use crate::convert::{model, spread_data};
use crate::state::App;
use crate::texts::SMALL;

pub fn refresh(app: &mut App) {
    let Some(w) = app.window.upgrade() else {
        return;
    };
    let editor = &app.editor;
    w.set_can_undo(editor.available(Command::Undo));
    w.set_can_redo(editor.available(Command::Redo));
    w.set_can_layout(editor.available(Command::NextLayout));
    w.set_can_left(editor.available(Command::NextLeft));
    w.set_can_right(editor.available(Command::NextRight));
    w.set_can_mirror(editor.available(Command::Mirror));
    w.set_can_mark(editor.available(Command::Favorite));
    w.set_layout_list(i32::from(editor.reordered()));
    w.set_has_reordered(editor.has_reordered());
    let Some(session) = editor.session() else {
        return;
    };
    let (project, index) = (session.project().clone(), session.current());
    let missing = session.missing().to_vec();

    let panel = app.editor.panel(Part::Spread);
    w.set_layout_in_use(
        panel
            .in_use
            .and_then(|p| i32::try_from(p).ok())
            .unwrap_or(-1),
    );
    let notice = match (&panel.unavailable, panel.all_rejected) {
        (Some(reason), _) => reason.to_string(),
        (None, true) => ALL_REJECTED.to_owned(),
        (None, false) => String::new(),
    };
    w.set_layout_notice(notice.into());
    // As mesmas prévias: nada a refazer, só a opção em uso muda (spec MV, regra 3; MV-B17).
    let key = panel_key(&panel, session);
    if app.lists.panel.as_ref() == Some(&key) {
        return;
    }
    let thumbs = app.thumbs(&project);
    let image = |id: PhotoId| thumbs.get(&id).cloned();
    let items: Vec<LayoutItem> = panel
        .options
        .iter()
        .enumerate()
        .filter_map(|(i, c)| {
            let preview = app.editor.preview(&panel, i)?;
            Some(LayoutItem {
                data: spread_data(&preview, index, &missing, &image, (false, SMALL)),
                title: option_title(&c.family).into(),
                score: format!("nota {:.0}", c.score.total).into(),
                favorite: panel.favorite.get(i).copied().unwrap_or(false),
            })
        })
        .collect();
    // Sem o número de opções nem a conta da nota: para o leigo, a miniatura basta (auditoria da
    // Fase 4, achado 26; decisão do dono).
    w.set_layout_count("".into());
    w.set_layouts(model(items));
    w.set_layout_explanation("".into());
    app.lists.panel = Some(key);
}
