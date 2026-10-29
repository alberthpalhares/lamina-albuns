//! Reencontrar fotos pela IA (spec IA, `reencontrar_fotos`; spec EI, regras 6 e 7; spec RF, regra
//! 9): procura as fotos faltando numa pasta permitida como Arquivo → Reencontrar fotos… (o nome
//! primeiro, o conteúdo só se precisar) e religa numa edição só.

use std::path::Path;

use serde_json::json;

use super::{Agent, Progress, RelinkCall, ToolError, ToolReply, session, session_mut};
use crate::{Editor, not_found_message, relinked_message, search_moved};

impl Agent {
    pub(super) fn relink(
        &mut self,
        editor: &mut Editor,
        a: &RelinkCall,
        progress: Progress<'_>,
    ) -> Result<ToolReply, ToolError> {
        let folder = self.roots.check(Path::new(&a.pasta))?;
        let s = session(editor)?;
        let missing = s.missing().to_vec();
        if missing.is_empty() {
            return Err(ToolError::NothingMissing);
        }
        let found = search_moved(s.project(), &missing, None, &folder, progress);
        let s = session_mut(editor)?;
        let relinked = s.relink(&found);
        if relinked == 0 {
            return Err(ToolError::Refused(not_found_message(
                missing.len(),
                &folder,
            )));
        }
        let still = s.missing().len();
        let text = relinked_message(relinked, still).unwrap_or_default();
        let data = json!({
            "reencontradas": relinked,
            "ainda_faltando": still,
            "alteracoes_nao_salvas": s.is_dirty(),
        });
        Ok(ToolReply::new(text, data))
    }
}
