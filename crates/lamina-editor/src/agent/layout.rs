//! Layouts pela IA (spec IA, regras 3 e 4): a lista da lâmina, aplicar uma opção dela e espelhar.
//! A lista é a do painel da janela (spec TL), com os favoritos e rejeitados do fotógrafo.

use serde_json::{Value, json};

use super::{
    Agent, ApplyLayout, Listed, OrderArg, PartArg, SuggestLayouts, ToolError, ToolReply, session,
    spread_index,
};
use crate::suggestions::{List, Part, can_mirror};
use crate::text::score_total;
use crate::{Command, Editor, option_title, score_text};
use lamina_layout::LayoutCandidate;

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
        let mut picks = picked(&panel.options, a.ordem, a.corte_maximo);
        if let (true, Some(max)) = (picks.is_empty(), a.corte_maximo) {
            return Err(ToolError::Refused(format!(
                "Nenhuma opção desta lâmina corta no máximo {max}% de cada foto. Aumente corte_maximo."
            )));
        }
        picks.truncate(a.quantas.unwrap_or(DEFAULT_OPTIONS).clamp(1, MAX_OPTIONS));
        let shown = picks.len();
        let options: Vec<Value> = picks
            .iter()
            .enumerate()
            .map(|(n, &i)| {
                let c = &panel.options[i];
                let fill = Fill::of(c);
                json!({
                    "opcao": n + 1,
                    "titulo": option_title(&c.family),
                    "nota": score_total(&c.score),
                    "nota_explicada": score_text(&c.score),
                    "area_ocupada": fill.area,
                    "menor_foto": fill.smallest,
                    "corte_maximo": fill.crop,
                    "favorita": panel.favorite.get(i).copied().unwrap_or(false),
                    "em_uso": panel.in_use == Some(i),
                })
            })
            .collect();
        let spread = session(editor)?.project().spreads[index].clone();
        let previews: Vec<usize> = picks.iter().copied().take(PREVIEWED).collect();
        self.lists.insert((index, part), Listed { spread, picks });
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
            for i in previews {
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
        let Some(&option) = a.opcao.checked_sub(1).and_then(|n| listed.picks.get(n)) else {
            return Err(ToolError::NoOption {
                asked: a.opcao,
                count: listed.picks.len(),
            });
        };
        editor.go_to_spread(index);
        if !editor.apply_option(part, option) {
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

/// As três medidas do preenchimento de uma opção, em % com uma casa (relato do teste pelo MCP,
/// IA-B27): a área dos quadros na lâmina, a menor foto e o maior corte de uma foto.
struct Fill {
    area: f64,
    smallest: f64,
    crop: f64,
}

impl Fill {
    fn of(c: &LayoutCandidate) -> Self {
        let smallest = c
            .frames
            .iter()
            .map(|f| f.width() * f.height())
            .fold(f64::INFINITY, f64::min);
        let crop = c.crop.iter().copied().fold(0.0, f64::max);
        Fill {
            area: percent(c.coverage),
            smallest: if smallest.is_finite() {
                percent(smallest)
            } else {
                0.0
            },
            crop: percent(crop),
        }
    }
}

fn percent(fraction: f64) -> f64 {
    (fraction * 1000.0).round() / 10.0
}

/// As posições das opções que entram na lista: as que cabem na tolerância de corte, na ordem da
/// janela ou da maior área para a menor (empate: a ordem da janela).
fn picked(options: &[LayoutCandidate], order: OrderArg, max_crop: Option<f64>) -> Vec<usize> {
    let fills: Vec<Fill> = options.iter().map(Fill::of).collect();
    let mut picks: Vec<usize> = (0..options.len())
        .filter(|&i| max_crop.is_none_or(|max| fills[i].crop <= max))
        .collect();
    if order == OrderArg::Preenchimento {
        picks.sort_by(|&a, &b| fills[b].area.total_cmp(&fills[a].area));
    }
    picks
}
