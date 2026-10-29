//! Tirar fotos do álbum pela IA (spec TF, regra 8): a mesma edição da grade, com o rótulo "IA: …".

use std::collections::HashSet;

use super::{Agent, LeaveAlbum, ToolError, ToolReply, photo_id, session, session_mut};
use crate::Editor;
use crate::leaving::{leave_label, remove_from_album};

impl Agent {
    pub(super) fn leave_album(
        &mut self,
        editor: &mut Editor,
        a: &LeaveAlbum,
    ) -> Result<ToolReply, ToolError> {
        if a.fotos.is_empty() {
            return Err(ToolError::NoPhotosChosen);
        }
        let p = session(editor)?.project();
        let mut seen = HashSet::new();
        let mut photos = Vec::new();
        for &n in &a.fotos {
            if !seen.insert(n) {
                return Err(ToolError::RepeatedPhoto(n));
            }
            photos.push(photo_id(p, n)?);
        }
        let label = leave_label(photos.len());
        session_mut(editor)?.edit(&label, |p| remove_from_album(p, &photos))?;
        let index = session(editor)?.current();
        let text = format!(
            "{} foto(s) tirada(s) do álbum; os arquivos continuam no computador.",
            photos.len()
        );
        self.edited(editor, index, &text)
    }
}
