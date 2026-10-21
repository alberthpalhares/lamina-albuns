//! `montar_album` (spec BR, regra 14): o mesmo que "Montar o álbum…" da janela, com a variedade
//! "Equilibrado", num passo do histórico "IA: Montar …".

use lamina_layout::Reuse;
use serde_json::json;

use super::{Agent, BuildAlbumCall, Progress, ToolError, ToolReply, session};
use crate::Editor;

impl Agent {
    pub(super) fn build_album(
        &mut self,
        editor: &mut Editor,
        a: &BuildAlbumCall,
        progress: Progress<'_>,
    ) -> Result<ToolReply, ToolError> {
        session(editor)?;
        let plan = editor
            .album_build()
            .map_err(|message| ToolError::Refused(message.to_owned()))?;
        let spreads = a.laminas.unwrap_or(plan.suggestion);
        let project = session(editor)?.project().clone();
        plan.check(&project, spreads)
            .map_err(|e| ToolError::Refused(e.to_string()))?;
        progress(0, spreads);
        let report = editor.build_album(Some(spreads), Reuse::Medium)?;
        progress(spreads, spreads);
        // As listas de sugerir_layouts eram do álbum de antes.
        self.lists.clear();
        let first = plan.kept + 1;
        let lines: Vec<String> = report
            .iter()
            .enumerate()
            .map(|(i, r)| {
                format!(
                    "Lâmina {}: {} fotos, {} ({})",
                    first + i,
                    r.photos,
                    crate::option_title(&r.family),
                    r.reason
                )
            })
            .collect();
        let s = session(editor)?;
        let text = format!(
            "{} lâminas novas com {} fotos, a partir da lâmina {first}: {}.",
            spreads,
            plan.unused.len(),
            s.counter()
        );
        Ok(ToolReply::new(
            text,
            json!({ "laminas_novas": spreads, "primeira": first, "relatorio": lines }),
        ))
    }
}
