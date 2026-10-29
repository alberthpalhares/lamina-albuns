//! Apagar fotos da lâmina (spec FQ, regra 3): a foto sai com o quadro e a parte da lâmina onde
//! ela estava ganha a 1ª opção do motor para as fotos que restaram.

use lamina_core::{FrameContent, Project, RelRect};

use crate::EditorError;
use crate::suggestions::{Part, apply, crosses_spine, lists, on_part, selection};

/// Tira as fotos dos quadros `frames` da lâmina `index`, com os quadros. Se as fotos estavam
/// numa página só (e nada passa da lombada), só aquela página muda; senão, a lâmina inteira.
/// Sem foto na parte, ela fica em branco: os textos ficam (spec FQ, regras 3 e 4).
pub fn remove_photos(
    project: &mut Project,
    index: usize,
    frames: &[usize],
) -> Result<(), EditorError> {
    let spread = project.spreads.get(index).ok_or(EditorError::NoFrame)?;
    let mut picked = frames.to_vec();
    picked.sort_unstable();
    picked.dedup();
    let rects = picked
        .iter()
        .map(|f| spread.frames.get(*f).map(|fr| fr.rect))
        .collect::<Option<Vec<RelRect>>>()
        .ok_or(EditorError::NoFrame)?;
    let crossing = spread
        .frames
        .iter()
        .any(|f| matches!(f.content, FrameContent::Photo { .. }) && crosses_spine(&f.rect));
    let part = part_of(&rects, crossing);
    let spread = &mut project.spreads[index];
    for f in picked.iter().rev() {
        spread.frames.remove(*f);
    }
    // Os quadros vazios da parte também saem: o layout novo é para as fotos (regra 4).
    spread
        .frames
        .retain(|f| !(f.content == FrameContent::Empty && on_part(&f.rect, part)));
    spread.manual = false;
    spread.model = None;
    let Ok(sel) = selection(project, index, part) else {
        return Ok(());
    };
    if let Some(first) = lists(project, &sel)?.in_order.first() {
        apply(project, index, &sel, first)?;
    }
    Ok(())
}

/// A página onde estão todos os quadros, ou a lâmina inteira.
fn part_of(rects: &[RelRect], crossing: bool) -> Part {
    if crossing {
        return Part::Spread;
    }
    [Part::Left, Part::Right]
        .into_iter()
        .find(|p| rects.iter().all(|r| on_part(r, *p)))
        .unwrap_or(Part::Spread)
}
