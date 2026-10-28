//! Arrastar fotos na faixa Sequência (spec SQ): onde a foto entra pelo `x` do ponteiro na linha e
//! a edição que leva a foto para outra lâmina ou muda a ordem dentro da mesma. As lâminas que
//! mudam são refeitas pelo motor, como no corte (FQ, regra 8).

use lamina_core::{PhotoId, Project};

use crate::EditorError;
use crate::edits::{layout_with, spread_photos};
use crate::edits_cross::check_room;
use crate::motion::{TimelineKey, TimelineSizes};

/// Onde a foto entra: na lâmina `spread`, antes da foto de posição `at` (`at` igual ao número de
/// fotos dela: no fim).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SeqSpot {
    pub spread: usize,
    pub at: usize,
}

/// O lugar de entrada para o ponteiro em `x` (pixels da linha inteira; regra 2): sobre uma foto,
/// antes ou depois dela pelo meio da miniatura; sobre um corte, no fim da lâmina da esquerda ou no
/// começo da da direita; depois da última, no fim da última lâmina.
pub fn sequence_spot(rows: &[(TimelineKey, usize)], x: f64, s: TimelineSizes) -> Option<SeqSpot> {
    let mut left = s.padding;
    let mut at = 0;
    for (key, spread) in rows {
        let wide = match key {
            TimelineKey::Photo(..) => s.photo,
            TimelineKey::Cut(_) => s.cut,
        };
        if x < left + wide / 2.0 {
            return Some(SeqSpot {
                spread: *spread,
                at,
            });
        }
        at = if matches!(key, TimelineKey::Cut(_)) {
            0
        } else {
            at + 1
        };
        left += wide + s.spacing;
    }
    rows.last().map(|(_, spread)| SeqSpot {
        spread: *spread,
        at,
    })
}

/// O `x` da linha de entrada do lugar `spot`, no meio do espaço antes do item que vem depois dele.
pub fn spot_x(rows: &[(TimelineKey, usize)], spot: SeqSpot, s: TimelineSizes) -> f64 {
    let mut left = s.padding;
    let mut at = 0;
    for (key, spread) in rows {
        let here = *spread == spot.spread && at == spot.at;
        if here {
            return left - s.spacing / 2.0;
        }
        let (wide, next) = match key {
            TimelineKey::Photo(..) => (s.photo, at + 1),
            TimelineKey::Cut(_) => (s.cut, 0),
        };
        at = next;
        left += wide + s.spacing;
    }
    left - s.spacing / 2.0
}

/// Leva a foto `photo` da lâmina `from` para o lugar `to` (regras 3 a 7). `Ok(false)`: soltou no
/// mesmo lugar, nada muda. Recusa a lâmina de origem que ficaria sem fotos, a cheia e a foto que
/// já está no destino.
pub fn move_in_sequence(
    project: &mut Project,
    from: usize,
    photo: PhotoId,
    to: SeqSpot,
) -> Result<bool, EditorError> {
    let before = spread_photos(project, from);
    let pos = before
        .iter()
        .position(|p| *p == photo)
        .ok_or(EditorError::NoFrame)?;
    let mut origin = before.clone();
    origin.remove(pos);
    if to.spread == from {
        let at = if to.at > pos { to.at - 1 } else { to.at };
        origin.insert(at.min(origin.len()), photo);
        if origin == before {
            return Ok(false);
        }
        layout_with(project, from, &origin)?;
        return Ok(true);
    }
    if origin.is_empty() {
        return Err(EditorError::LastPhoto { spread: from });
    }
    let mut dest = spread_photos(project, to.spread);
    check_room(project, to.spread, &dest, &[photo])?;
    dest.insert(to.at.min(dest.len()), photo);
    layout_with(project, from, &origin)?;
    layout_with(project, to.spread, &dest)?;
    Ok(true)
}
