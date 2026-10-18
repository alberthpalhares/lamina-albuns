//! Aparência (spec AP): espaço entre fotos mantendo o desenho, fundo, borda e cantos. Funções
//! puras sobre o projeto, chamadas dentro de [`crate::Session::edit`].

use lamina_core::{Border, FrameContent, Mm, PageSide, Project, Rgb};
use lamina_layout::{LayoutError, design_key};

use crate::EditorError;
use crate::suggestions::{Part, apply, is_in_use, lists, selection};

/// Maior espaço entre fotos aceito (regra da seção 3).
pub const MAX_GAP: Mm = Mm(20.0);
/// Passo com que o espaço é reduzido quando não cabe (regra 1, seção 6).
const GAP_STEP: f64 = 0.5;

/// Refaz a lâmina `index` com o espaço `gap`, mantendo o desenho em uso (mesmo `design_key`)
/// quando ele existe com o espaço novo; senão, a 1ª opção. Se não couber, reduz de 0,5 em
/// 0,5 mm. Devolve o espaço que ficou (regra 1).
pub fn regap(project: &mut Project, index: usize, gap: Mm) -> Result<Mm, EditorError> {
    let gap = Mm(gap.get().clamp(0.0, MAX_GAP.get()));
    let manual = project.spreads.get(index).is_some_and(|s| s.manual);
    // Lâmina à mão: o espaço fica guardado, mas não mexe nos quadros (spec TX, regra 8).
    let old = match selection(project, index, Part::Spread) {
        Ok(old) if !manual => old,
        // Lâmina sem fotos, ou à mão: só guarda o espaço.
        _ => {
            set_gap(project, index, gap)?;
            return Ok(gap);
        }
    };
    let before = lists(project, &old)?;
    let key = before
        .in_order
        .iter()
        .chain(&before.reordered)
        .find(|c| is_in_use(project, index, &old, c))
        .map(|c| design_key(c, &old.aspects));
    let mut tried = gap.get();
    loop {
        let sel = crate::suggestions::Selection {
            gap: Mm(tried),
            ..old.clone()
        };
        match lists(project, &sel) {
            Ok(new) if !new.in_order.is_empty() => {
                let all: Vec<_> = new.in_order.iter().chain(&new.reordered).collect();
                let same = key
                    .as_ref()
                    .and_then(|k| all.iter().find(|c| design_key(c, &sel.aspects) == *k));
                let chosen = (*same.unwrap_or(&all[0])).clone();
                set_gap(project, index, Mm(tried))?;
                apply(project, index, &sel, &chosen)?;
                return Ok(Mm(tried));
            }
            Ok(_) | Err(LayoutError::GapTooLarge) if tried > 0.0 => {
                tried = (tried - GAP_STEP).max(0.0);
            }
            Ok(_) => return Err(EditorError::NoFrame),
            Err(e) => return Err(e.into()),
        }
    }
}

fn set_gap(project: &mut Project, index: usize, gap: Mm) -> Result<(), EditorError> {
    let spread = project.spreads.get_mut(index).ok_or(EditorError::NoFrame)?;
    spread.gap = Some(gap);
    Ok(())
}

/// Fundo da lâmina inteira (`side` = `None`) ou de uma página (regra 3).
pub fn set_background(
    project: &mut Project,
    index: usize,
    color: Rgb,
    side: Option<PageSide>,
) -> Result<(), EditorError> {
    let spread = project.spreads.get_mut(index).ok_or(EditorError::NoFrame)?;
    match side {
        None => {
            spread.background = color;
            spread.background_right = None;
        }
        Some(PageSide::Left) => {
            let right = spread.page_background(PageSide::Right);
            spread.background = color;
            spread.background_right = Some(right);
        }
        Some(PageSide::Right) => spread.background_right = Some(color),
    }
    Ok(())
}

/// Borda e cantos: do quadro `frame`, ou de todas as fotos da lâmina (`None`; regra 6).
pub fn set_frame_style(
    project: &mut Project,
    index: usize,
    frame: Option<usize>,
    border: Option<Border>,
    radius: Option<Mm>,
) -> Result<(), EditorError> {
    let spread = project.spreads.get_mut(index).ok_or(EditorError::NoFrame)?;
    let photo_frames = spread.frames.iter_mut().enumerate().filter(|(i, f)| {
        frame.is_none_or(|only| only == *i) && matches!(f.content, FrameContent::Photo { .. })
    });
    for (_, f) in photo_frames {
        if let Some(b) = border {
            f.border = b;
        }
        if let Some(r) = radius {
            f.radius = r;
        }
    }
    Ok(())
}

/// Copia espaço, fundo, borda e cantos da lâmina `from` para todas (regra 6), e o espaço vira o
/// do álbum (regra 2).
pub fn style_all(project: &mut Project, from: usize) -> Result<(), EditorError> {
    let source = project
        .spreads
        .get(from)
        .ok_or(EditorError::NoFrame)?
        .clone();
    let gap = project.spread_gap(&source);
    let first = source
        .frames
        .iter()
        .find(|f| matches!(f.content, FrameContent::Photo { .. }));
    let (border, radius) = first.map_or((Border::default(), Mm(0.0)), |f| (f.border, f.radius));
    // O espaço que cada lâmina tinha, antes de o do álbum mudar.
    let old: Vec<Mm> = project
        .spreads
        .iter()
        .map(|s| project.spread_gap(s))
        .collect();
    for (index, before) in old.into_iter().enumerate() {
        project.spreads[index].background = source.background;
        project.spreads[index].background_right = source.background_right;
        if (before.get() - gap.get()).abs() > 1e-9 {
            regap(project, index, gap)?;
        }
        project.spreads[index].gap = None;
        set_frame_style(project, index, None, Some(border), Some(radius))?;
    }
    project.gap = gap;
    Ok(())
}
