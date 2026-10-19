//! Layouts pela IA (spec IA, regras 3 e 4): a lista da lâmina, aplicar uma opção dela e espelhar.
//! A lista é a do painel da janela (spec TL), com os favoritos e rejeitados do fotógrafo.

use serde_json::{Value, json};

use super::{
    Agent, ApplyLayout, Listed, PartArg, SuggestLayouts, ToolError, ToolReply, session,
    spread_index,
};
use crate::suggestions::{List, Part, can_mirror};
use crate::text::score_total;
use crate::{Command, Editor, option_title, score_text};

/// Quantas opções sem `quantas`, e o máximo (regra 9).
const DEFAULT_OPTIONS: usize = 6;
const MAX_OPTIONS: usize = 24;
/// Só as primeiras opções vão com prévia, de 512 px (seção 4).
const PREVIEWED: usize = 6;
const OPTION_SIDE: u32 = 512;

fn part_of(arg: PartArg) -> Part {
    match arg {
        PartArg::Lamina => Part::Spread,
        PartArg::Esquerda => Part::Left,
        PartArg::Direita => Part::Right,
    }
}

impl Agent {
    pub(super) fn suggest(
        &mut self,
        editor: &mut Editor,
        a: &SuggestLayouts,
    ) -> Result<ToolReply, ToolError> {
        let index = spread_index(session(editor)?.project(), a.lamina)?;
        editor.go_to_spread(index);
        let part = part_of(a.parte);
        let panel = editor.panel(part);
        if let Some(reason) = &panel.unavailable {
            return Err(ToolError::Refused(reason.to_string()));
        }
        let shown = a
            .quantas
            .unwrap_or(DEFAULT_OPTIONS)
            .clamp(1, MAX_OPTIONS)
            .min(panel.options.len());
        let options: Vec<Value> = panel
            .options
            .iter()
            .take(shown)
            .enumerate()
            .map(|(i, c)| {
                json!({
                    "opcao": i + 1,
                    "titulo": option_title(&c.family),
                    "nota": score_total(&c.score),
                    "nota_explicada": score_text(&c.score),
                    "favorita": panel.favorite.get(i).copied().unwrap_or(false),
                    "em_uso": panel.in_use == Some(i),
                })
            })
            .collect();
        let spread = session(editor)?.project().spreads[index].clone();
        self.lists.insert(
            (index, part),
            Listed {
                spread,
                count: shown,
            },
        );
        let list = match panel.list {
            List::InOrder => "na ordem",
            List::Reordered => "reordenadas",
        };
        let data = json!({
            "lamina": a.lamina,
            "lista": list,
            "todas_rejeitadas": panel.all_rejected,
            "opcoes": options,
        });
        let mut reply = ToolReply::new(
            format!("{shown} opções de layout para a lâmina {}.", a.lamina),
            data,
        );
        if a.imagens && self.images {
            for i in 0..shown.min(PREVIEWED) {
                if let Some(project) = editor.preview(&panel, i) {
                    reply
                        .images
                        .push(self.preview(&project, index, OPTION_SIDE)?);
                }
            }
        }
        Ok(reply)
    }

    /// Aplica uma opção da última lista, se a lâmina não mudou desde ela (regra 3).
    pub(super) fn apply(
        &mut self,
        editor: &mut Editor,
        a: &ApplyLayout,
    ) -> Result<ToolReply, ToolError> {
        let index = spread_index(session(editor)?.project(), a.lamina)?;
        let part = part_of(a.parte);
        let listed = self
            .lists
            .get(&(index, part))
            .ok_or(ToolError::NoList(a.lamina))?;
        if session(editor)?.project().spreads[index] != listed.spread {
            return Err(ToolError::StaleList(a.lamina));
        }
        if a.opcao == 0 || a.opcao > listed.count {
            return Err(ToolError::NoOption {
                asked: a.opcao,
                count: listed.count,
            });
        }
        editor.go_to_spread(index);
        if !editor.apply_option(part, a.opcao - 1) {
            let why = editor.take_notice();
            return Err(ToolError::Refused(
                why.unwrap_or_else(|| "Não deu para aplicar esse layout.".into()),
            ));
        }
        self.lists.retain(|(i, _), _| *i != index);
        self.edited(
            editor,
            index,
            &format!("Layout {} aplicado na lâmina {}.", a.opcao, a.lamina),
        )
    }

    pub(super) fn mirror(
        &mut self,
        editor: &mut Editor,
        lamina: usize,
    ) -> Result<ToolReply, ToolError> {
        let s = session(editor)?;
        let index = spread_index(s.project(), lamina)?;
        can_mirror(s.project(), index).map_err(|r| ToolError::Refused(r.to_string()))?;
        editor.go_to_spread(index);
        editor.run(Command::Mirror);
        self.edited(editor, index, &format!("Lâmina {lamina} espelhada."))
    }
}
