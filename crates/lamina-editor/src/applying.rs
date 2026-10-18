//! Aplicar uma opção de layout na lâmina (spec TL, regras 3 e 4): as fotos da seleção vão para
//! os quadros do candidato, com borda, cantos e, num modelo do fotógrafo, os textos dele (spec MD).

use lamina_core::{Border, CoreError, Crop, Frame, FrameContent, Mm, PhotoId, Project, RelRect};
use lamina_layout::LayoutCandidate;

use crate::suggestions::{Selection, photo_of};

/// Troca o layout da `part` da lâmina `index` pelo candidato: as fotos da seleção vão para os
/// quadros do candidato (na ordem original), com o recorte no centro; o resto fica (regra 3 e 4).
pub fn apply(
    project: &mut Project,
    index: usize,
    sel: &Selection,
    c: &LayoutCandidate,
) -> Result<(), CoreError> {
    let Some(spread) = project.spreads.get_mut(index) else {
        return Ok(());
    };
    let id = spread.id;
    let centers: Vec<(PhotoId, Crop)> = spread
        .frames
        .iter()
        .filter_map(|f| match &f.content {
            FrameContent::Photo { photo, crop } => Some((*photo, keep_center(*crop))),
            _ => None,
        })
        .collect();
    // Borda e cantos ficam com a foto; foto nova herda o estilo da lâmina (o do primeiro quadro
    // com foto). Antes, trocar o layout ou o espaço apagava os dois (auditoria do Marco 2, M7).
    let styles: Vec<(PhotoId, Border, Mm)> = spread
        .frames
        .iter()
        .filter_map(|f| photo_of(f).map(|p| (p, f.border, f.radius)))
        .collect();
    let common = styles
        .first()
        .map_or((Border::default(), Mm(0.0)), |(_, b, r)| (*b, *r));
    let style_of = |photo: Option<PhotoId>| {
        photo
            .and_then(|p| styles.iter().find(|(q, _, _)| *q == p))
            .map_or(common, |(_, b, r)| (*b, *r))
    };
    let kept: Vec<Frame> = std::mem::take(&mut spread.frames)
        .into_iter()
        .filter(|f| !photo_of(f).is_some_and(|p| sel.photos.contains(&p)))
        .filter(|f| !(sel.whole && f.content == FrameContent::Empty))
        .collect();
    let new = sel.photos.iter().zip(&c.frames).map(|(photo, rect)| {
        let crop = centers
            .iter()
            .find(|(p, _)| p == photo)
            .map_or_else(Crop::default, |(_, c)| *c);
        let content = FrameContent::Photo {
            photo: *photo,
            crop,
        };
        (*rect, content)
    });
    let kept = kept.into_iter().map(|f| (f.rect, f.content));
    // Ordem dos quadros: página esquerda, depois a direita (a ordem em que o TL lê as fotos).
    let only_left = sel.active.left && !sel.active.right;
    let ordered: Vec<(RelRect, FrameContent)> = if only_left {
        new.chain(kept).collect()
    } else {
        kept.chain(new).collect()
    };
    // Num modelo do fotógrafo, borda e cantos vêm dele, foto a foto (spec MD, regra 6).
    let look = c.model.as_ref();
    let model_style = |photo: PhotoId| {
        let at = sel.photos.iter().position(|p| *p == photo)?;
        look?.styles.get(at).copied()
    };
    for (rect, content) in ordered {
        let text = matches!(content, FrameContent::Text(_));
        let photo = match &content {
            FrameContent::Photo { photo, .. } => Some(*photo),
            _ => None,
        };
        let (border, radius) = photo
            .and_then(model_style)
            .unwrap_or_else(|| style_of(photo));
        project.add_frame(id, rect, content)?;
        if let Some(frame) = project
            .spreads
            .get_mut(index)
            .and_then(|s| s.frames.last_mut())
            .filter(|_| !text)
        {
            frame.border = border;
            frame.radius = radius;
        }
    }
    finish(project, index, sel, c)
}

/// Depois de aplicar: a lâmina deixa de ser à mão (spec TX, regra 8), guarda o modelo em uso e
/// ganha os textos do modelo se não tinha texto (spec MD, regra 6).
fn finish(
    project: &mut Project,
    index: usize,
    sel: &Selection,
    c: &LayoutCandidate,
) -> Result<(), CoreError> {
    let Some(spread) = project.spreads.get_mut(index) else {
        return Ok(());
    };
    spread.manual = false;
    spread.model = c
        .model
        .as_ref()
        .map(|_| lamina_layout::design_key(c, &sel.aspects));
    let has_text = spread
        .frames
        .iter()
        .any(|f| matches!(f.content, FrameContent::Text(_)));
    let id = spread.id;
    if let Some(look) = c.model.as_ref().filter(|_| !has_text) {
        for t in &look.texts {
            project.add_frame(id, t.rect, FrameContent::Text(t.text.clone()))?;
        }
    }
    Ok(())
}

/// A mesma foto num quadro de outra proporção: mantém o centro do recorte e volta o zoom a 1
/// (spec RC, regra 8).
pub fn keep_center(crop: Crop) -> Crop {
    Crop { zoom: 1.0, ..crop }
}
