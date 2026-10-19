//! Reencontrar fotos pela IA (spec IA, `reencontrar_fotos`; spec EI, regras 6 e 7): procura as
//! fotos faltando numa pasta permitida pela impressão digital, como Arquivo → Procurar fotos
//! ausentes…, e religa numa edição só.

use std::path::Path;

use serde_json::json;

use super::{Agent, Progress, RelinkCall, ToolError, ToolReply, session, session_mut};
use crate::{Editor, not_found_message, relinked_message, search_folder};

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
        let found = search_folder(s.project(), &missing, &folder, progress);
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
