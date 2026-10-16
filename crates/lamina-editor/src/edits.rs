//! Edições de fotos e lâminas (spec FQ, regras 1 a 8): funções puras sobre o projeto, chamadas
//! pela sessão dentro de [`crate::Session::edit`] para entrarem no histórico.

use lamina_core::{Crop, FrameContent, PhotoId, Project};
use lamina_layout::{LayoutOptions, MAX_PER_PAGE, layout_options};

use crate::EditorError;

/// Só a lista na ordem: a 1ª opção dela é a mesma da lista completa, sem calcular as reordenadas.
const FIRST_ONLY: LayoutOptions = LayoutOptions {
    gap: lamina_layout::DEFAULT_GAP,
    max_candidates: 12,
    reordered: false,
};

fn photo_of(content: &FrameContent) -> Option<PhotoId> {
    match content {
        FrameContent::Photo { photo, .. } => Some(*photo),
        _ => None,
    }
}

/// As fotos dos quadros da lâmina, na ordem dos quadros, sem repetir.
pub fn spread_photos(project: &Project, index: usize) -> Vec<PhotoId> {
    let mut out = Vec::new();
    for f in project.spreads.get(index).map_or(&[][..], |s| &s.frames) {
        if let Some(p) = photo_of(&f.content)
            && !out.contains(&p)
        {
            out.push(p);
        }
    }
    out
}

/// Quantas fotos cabem na lâmina `index` (até 12 por página ativa).
pub fn capacity(project: &Project, index: usize) -> usize {
    let active = project
        .spec
        .edges()
        .active_pages(index, project.spreads.len());
    MAX_PER_PAGE * (usize::from(active.left) + usize::from(active.right))
}

/// Põe `photo` no quadro `frame` da lâmina `spread`, com o recorte no centro (regra 1).
pub fn place_photo(
    project: &mut Project,
    spread: usize,
    frame: usize,
    photo: PhotoId,
) -> Result<(), EditorError> {
    project.photo(photo)?;
    let f = project
        .spreads
        .get_mut(spread)
        .and_then(|s| s.frames.get_mut(frame))
        .ok_or(EditorError::NoFrame)?;
    f.content = FrameContent::Photo {
        photo,
        crop: Crop::default(),
    };
    Ok(())
}

/// Troca as fotos de dois quadros da mesma lâmina; os quadros não mudam (regra 2).
pub fn swap_photos(
    project: &mut Project,
    spread: usize,
    a: usize,
    b: usize,
) -> Result<(), EditorError> {
    let s = project
        .spreads
        .get_mut(spread)
        .ok_or(EditorError::NoFrame)?;
    if a >= s.frames.len() || b >= s.frames.len() || a == b {
        return Err(EditorError::NoFrame);
    }
    let reset = |c: FrameContent| match c {
        FrameContent::Photo { photo, crop } => FrameContent::Photo {
            photo,
            crop: crate::suggestions::keep_center(crop),
        },
        other => other,
    };
    let (ca, cb) = (s.frames[a].content.clone(), s.frames[b].content.clone());
    s.frames[a].content = reset(cb);
    s.frames[b].content = reset(ca);
    Ok(())
}

/// Deixa o quadro vazio (regra 3).
pub fn clear_frame(project: &mut Project, spread: usize, frame: usize) -> Result<(), EditorError> {
    let f = project
        .spreads
        .get_mut(spread)
        .and_then(|s| s.frames.get_mut(frame))
        .ok_or(EditorError::NoFrame)?;
    f.content = FrameContent::Empty;
    Ok(())
}

/// Arranja a lâmina `index` só com `photos`, na 1ª opção do motor (sem fotos, fica vazia).
pub fn layout_with(
    project: &mut Project,
    index: usize,
    photos: &[PhotoId],
) -> Result<(), EditorError> {
    let n = project.spreads.len();
    let id = project.spreads.get(index).ok_or(EditorError::NoFrame)?.id;
    let mut frames = Vec::new();
    if !photos.is_empty() {
        let aspects = photos
            .iter()
            .map(|p| Ok(project.photo(*p)?.aspect_ratio()))
            .collect::<Result<Vec<f64>, EditorError>>()?;
        let active = project.spec.edges().active_pages(index, n);
        let lists = layout_options(&project.spec, active, &aspects, &FIRST_ONLY)?;
        let first = lists
            .in_order
            .into_iter()
            .next()
            .ok_or(EditorError::NoFrame)?;
        frames = photos.iter().copied().zip(first.frames).collect();
    }
    project.spread_mut(id)?.frames.clear();
    for (photo, rect) in frames {
        let content = FrameContent::Photo {
            photo,
            crop: Crop::default(),
        };
        project.add_frame(id, rect, content)?;
    }
    Ok(())
}

/// Lâmina vazia nova depois de `after`; devolve a posição dela (regra 6).
pub fn insert_spread(project: &mut Project, after: usize) -> usize {
    project.add_spread();
    let at = (after + 1).min(project.spreads.len() - 1);
    if let Some(new) = project.spreads.pop() {
        project.spreads.insert(at, new);
    }
    at
}

/// Apaga a lâmina `index`; o álbum fica com pelo menos 1 (regra 6).
pub fn delete_spread(project: &mut Project, index: usize) -> Result<(), EditorError> {
    if project.spreads.len() <= 1 || index >= project.spreads.len() {
        return Err(EditorError::NoFrame);
    }
    project.spreads.remove(index);
    fix_edges(project)
}

/// Muda a lâmina `from` para a posição `to` (regra 7).
pub fn move_spread(project: &mut Project, from: usize, to: usize) -> Result<(), EditorError> {
    let n = project.spreads.len();
    if from >= n || to >= n || from == to {
        return Err(EditorError::NoFrame);
    }
    let before = pages_by_id(project);
    let spread = project.spreads.remove(from);
    project.spreads.insert(to, spread);
    fix_changed(project, &before)
}

type Pages = Vec<(lamina_core::SpreadId, (bool, bool))>;

fn pages_by_id(project: &Project) -> Pages {
    let edges = project.spec.edges();
    let n = project.spreads.len();
    project
        .spreads
        .iter()
        .enumerate()
        .map(|(i, s)| {
            let a = edges.active_pages(i, n);
            (s.id, (a.left, a.right))
        })
        .collect()
}

/// Rearranja as lâminas cujas páginas ativas mudaram (chegaram ou saíram de uma ponta).
fn fix_changed(project: &mut Project, before: &Pages) -> Result<(), EditorError> {
    let after = pages_by_id(project);
    for (index, (id, pages)) in after.iter().enumerate() {
        let old = before.iter().find(|(b, _)| b == id).map(|(_, p)| *p);
        if old.is_some_and(|o| o != *pages) {
            let photos = spread_photos(project, index);
            layout_with(project, index, &photos)?;
        }
    }
    Ok(())
}

/// Depois de apagar, as novas pontas podem ter mudado de páginas ativas.
fn fix_edges(project: &mut Project) -> Result<(), EditorError> {
    let last = project.spreads.len() - 1;
    for index in [0, last] {
        let active = project
            .spec
            .edges()
            .active_pages(index, project.spreads.len());
        let on_inactive = project.spreads[index].frames.iter().any(|f| {
            let left = f.rect.x() + f.rect.width() <= 0.5 + 1e-6;
            let right = f.rect.x() >= 0.5 - 1e-6;
            (!active.left && !right) || (!active.right && !left)
        });
        if on_inactive {
            let photos = spread_photos(project, index);
            layout_with(project, index, &photos)?;
        }
    }
    Ok(())
}

/// As fotos de cada lâmina, na ordem do álbum (a timeline; regra 8).
pub fn timeline(project: &Project) -> Vec<Vec<PhotoId>> {
    (0..project.spreads.len())
        .map(|i| spread_photos(project, i))
        .collect()
}

/// Move o corte entre as lâminas `k` e `k + 1`: `delta` < 0 passa as últimas fotos de `k` para o
/// começo de `k + 1`; `delta` > 0, as primeiras de `k + 1` para o fim de `k`. Para nos limites
/// (nenhuma lâmina vazia ou cheia demais). `false` se nada mudou.
pub fn move_cut(project: &mut Project, k: usize, delta: i32) -> Result<bool, EditorError> {
    if k + 1 >= project.spreads.len() || delta == 0 {
        return Ok(false);
    }
    let (mut a, mut b) = (spread_photos(project, k), spread_photos(project, k + 1));
    let want = usize::try_from(delta.unsigned_abs()).unwrap_or(usize::MAX);
    let n = if delta < 0 {
        want.min(a.len().saturating_sub(1))
            .min(capacity(project, k + 1).saturating_sub(b.len()))
    } else {
        want.min(b.len().saturating_sub(1))
            .min(capacity(project, k).saturating_sub(a.len()))
    };
    if n == 0 {
        return Ok(false);
    }
    if delta < 0 {
        let moved = a.split_off(a.len() - n);
        b.splice(0..0, moved);
    } else {
        let rest = b.split_off(n);
        a.extend(b);
        b = rest;
    }
    layout_with(project, k, &a)?;
    layout_with(project, k + 1, &b)?;
    Ok(true)
}
