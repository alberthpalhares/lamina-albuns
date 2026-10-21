//! O fantasma do arrasto (spec MV, regra 12): a parte da foto que está no quadro, com o selo da
//! quantidade. As contas são do editor (`frame_ghost`, `photo_ghost`); aqui só a imagem.

use lamina_editor::motion::{Ghost, frame_ghost, photo_ghost};
use slint::Image;

use crate::GhostData;
use crate::state::App;

/// O fantasma do arrasto que começou: `kind` 1 = foto do navegador (`from` = posição nele), 2 =
/// quadro da lâmina atual (`from` = quadro).
pub fn make(app: &mut App, kind: i32, from: i32) -> Option<GhostData> {
    let from = usize::try_from(from).ok()?;
    let project = app.session()?.project().clone();
    let index = app.session()?.current();
    let ghost = match kind {
        1 => {
            let id = app.photo_at(i32::try_from(from).ok()?)?;
            photo_ghost(&project, id, app.editor.selection())?
        }
        2 => frame_ghost(&project, index, from)?,
        _ => return None,
    };
    let img = app
        .images
        .spread(&project, index)
        .get(&ghost.photo)
        .cloned()
        .or_else(|| app.images.thumb(ghost.photo))?;
    Some(data(&ghost, img))
}

// Pixels e frações da tela cabem com folga em i32 e f32.
#[allow(clippy::cast_possible_truncation)]
fn data(ghost: &Ghost, img: Image) -> GhostData {
    let size = img.size();
    let px = |f: f64, side: u32| (f * f64::from(side)).round() as i32;
    GhostData {
        cx: px(ghost.source.x, size.width),
        cy: px(ghost.source.y, size.height),
        cw: px(ghost.source.width, size.width).max(1),
        ch: px(ghost.source.height, size.height).max(1),
        w: ghost.width as f32,
        h: ghost.height as f32,
        badge: ghost.badge.and_then(|n| i32::try_from(n).ok()).unwrap_or(0),
        img,
    }
}
