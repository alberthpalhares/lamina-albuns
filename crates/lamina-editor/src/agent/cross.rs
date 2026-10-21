//! Fotos entre lâminas pela IA (spec ML, regra 14): `trocar_fotos` com `lamina_b` e `mover_foto`.
//! As mesmas edições da janela (`edits_cross.rs`), um passo "IA: …" cada; se falha, nada muda. A
//! resposta traz as duas lâminas, e a lâmina atual da IA passa a ser a de destino.

use serde_json::json;

use lamina_core::FrameContent;

use super::read::spread_data;
use super::{
    Agent, MovePhoto, ToolError, ToolReply, photo_frame, session, session_mut, spread_index,
};
use crate::Editor;
use crate::edits_cross::{Moved, move_to_frame, move_to_spread};

/// O quadro `frame` (posição) da lâmina `spread` tem foto?
fn has_photo(editor: &Editor, spread: usize, frame: usize) -> Result<bool, ToolError> {
    let p = session(editor)?.project();
    Ok(matches!(
        p.spreads[spread].frames[frame].content,
        FrameContent::Photo { .. }
    ))
}

impl Agent {
    /// Troca as fotos de dois quadros de lâminas diferentes (o destino vazio enche).
    pub(super) fn swap_across(
        &mut self,
        editor: &mut Editor,
        (a, qa): (usize, usize),
        (b, qb): (usize, usize),
    ) -> Result<ToolReply, ToolError> {
        let p = session(editor)?.project();
        let (sa, sb) = (spread_index(p, a)?, spread_index(p, b)?);
        let (fa, fb) = (photo_frame(p, sa, qa)?, photo_frame(p, sb, qb)?);
        // Quem tem foto vai para o outro; com as duas vazias, não há o que trocar.
        let (from, to) = if has_photo(editor, sa, fa)? {
            ((sa, fa), (sb, fb))
        } else if has_photo(editor, sb, fb)? {
            ((sb, fb), (sa, fa))
        } else {
            return Err(ToolError::NothingToMove {
                spread: a,
                frame: qa,
            });
        };
        let label = format!("Trocar fotos · lâminas {a} e {b}");
        self.cross_edit(editor, &label, (sa, sb), |p| {
            move_to_frame(p, from, to).map(|_| ())
        })?;
        let text =
            format!("Fotos do quadro {qa} da lâmina {a} e do quadro {qb} da lâmina {b} trocadas.");
        self.two_spreads(editor, (sa, sb), &text)
    }

    /// `mover_foto`: com `para_quadro`, enche ou troca; sem, acrescenta (regra 14).
    pub(super) fn move_photo(
        &mut self,
        editor: &mut Editor,
        a: &MovePhoto,
    ) -> Result<ToolReply, ToolError> {
        let p = session(editor)?.project();
        let origin = spread_index(p, a.lamina)?;
        let frame = photo_frame(p, origin, a.quadro)?;
        let dest = spread_index(p, a.para_lamina)?;
        if !has_photo(editor, origin, frame)? {
            return Err(ToolError::NothingToMove {
                spread: a.lamina,
                frame: a.quadro,
            });
        }
        let label = format!("Mover foto · lâmina {} → {}", a.lamina, a.para_lamina);
        let text = match a.para_quadro {
            Some(q) => {
                let target = photo_frame(p, dest, q)?;
                if (origin, frame) == (dest, target) {
                    return Err(ToolError::SameFrame);
                }
                let swaps = has_photo(editor, dest, target)?;
                let label = if swaps {
                    format!("Trocar fotos · lâminas {} e {}", a.lamina, a.para_lamina)
                } else {
                    label
                };
                self.cross_edit(editor, &label, (origin, dest), |p| {
                    move_to_frame(p, (origin, frame), (dest, target)).map(|_: Moved| ())
                })?;
                format!(
                    "Foto do quadro {} da lâmina {} no quadro {q} da lâmina {}.",
                    a.quadro, a.lamina, a.para_lamina
                )
            }
            None => {
                self.cross_edit(editor, &label, (origin, dest), |p| {
                    move_to_spread(p, (origin, frame), dest)
                })?;
                format!(
                    "Foto do quadro {} da lâmina {} acrescentada à lâmina {}.",
                    a.quadro, a.lamina, a.para_lamina
                )
            }
        };
        self.two_spreads(editor, (origin, dest), &text)
    }

    /// A edição entre duas lâminas, num passo só; a lâmina atual vira a de destino.
    fn cross_edit(
        &mut self,
        editor: &mut Editor,
        label: &str,
        (_, dest): (usize, usize),
        change: impl FnOnce(&mut lamina_core::Project) -> Result<(), crate::EditorError>,
    ) -> Result<(), ToolError> {
        session_mut(editor)?.edit(label, change)?;
        editor.go_to_spread(dest);
        Ok(())
    }

    /// A resposta com as duas lâminas atualizadas e "alterações não salvas".
    fn two_spreads(
        &self,
        editor: &Editor,
        (a, b): (usize, usize),
        text: &str,
    ) -> Result<ToolReply, ToolError> {
        let s = session(editor)?;
        let data = json!({
            "laminas": [spread_data(s, a), spread_data(s, b)],
            "alteracoes_nao_salvas": s.is_dirty(),
        });
        Ok(ToolReply::new(text, data))
    }
}
