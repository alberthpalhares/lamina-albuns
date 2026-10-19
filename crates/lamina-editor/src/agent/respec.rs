//! Trocar a especificação pela IA (spec IA, `trocar_especificacao`; spec PR, regras 6 e 7): um
//! preset ou as medidas da janela Especificação, numa edição só, que um desfazer volta inteira.

use serde_json::json;

use super::{Agent, RespecCall, ToolError, ToolReply, session, session_mut};
use crate::{Editor, Preset, PresetError, SpecFields};

const NOTHING: &str = "Diga o preset ou as medidas: largura_cm, altura_cm, dpi, sangria_mm, \
                       zona_segura_mm, lombada_mm, inicia_com_pagina, termina_com_pagina, \
                       paginas_min ou paginas_max.";

impl Agent {
    /// O preset de nome `name` (sem diferença de maiúsculas), do Lamina ou do fotógrafo.
    pub(super) fn preset(&self, name: &str) -> Result<&Preset, ToolError> {
        self.presets
            .iter()
            .find(|p| p.name.eq_ignore_ascii_case(name.trim()))
            .ok_or_else(|| ToolError::NoPreset {
                name: name.to_owned(),
                known: self
                    .presets
                    .iter()
                    .map(|p| p.name.as_str())
                    .collect::<Vec<_>>()
                    .join(", "),
            })
    }

    pub(super) fn respec(
        &mut self,
        editor: &mut Editor,
        a: &RespecCall,
    ) -> Result<ToolReply, ToolError> {
        let p = session(editor)?.project();
        let preset = a.preset.as_deref().map(|n| self.preset(n)).transpose()?;
        let (mut fields, mut limits) = match preset {
            Some(pr) => (pr.fields.clone(), pr.limits),
            None => (SpecFields::from_spec(&p.spec), p.page_limits),
        };
        if !measures(a, &mut fields)
            && preset.is_none()
            && a.paginas_min.is_none()
            && a.paginas_max.is_none()
        {
            return Err(ToolError::NothingAsked(NOTHING));
        }
        limits.min = a.paginas_min.or(limits.min);
        limits.max = a.paginas_max.or(limits.max);
        if let (Some(min), Some(max)) = (limits.min, limits.max)
            && min > max
        {
            return Err(ToolError::Refused(PresetError::LimitsOrder.to_string()));
        }
        let spec = fields
            .to_spec()
            .map_err(|e| ToolError::Refused(e.to_string()))?;
        let export = preset.map(|pr| pr.export);
        let notice = editor.respec(spec, limits)?;
        // Como a janela: escolher um preset traz também as preferências de exportação dele.
        if let Some(export) = export {
            session_mut(editor)?.set_export_prefs(export);
        }
        self.lists.clear();
        let mut text = format!(
            "Especificação trocada para {} × {} cm em todas as lâminas.",
            crate::frame_form::decimal(fields.width_cm),
            crate::frame_form::decimal(fields.height_cm)
        );
        if let Some(n) = &notice {
            text.push(' ');
            text.push_str(n);
        }
        let s = session(editor)?;
        let data = json!({
            "especificacao": fields,
            "paginas": { "min": limits.min, "max": limits.max },
            "aviso": notice,
            "alteracoes_nao_salvas": s.is_dirty(),
        });
        Ok(ToolReply::new(text, data))
    }
}

/// Põe nos campos as medidas pedidas; `false` se nenhuma foi pedida.
fn measures(a: &RespecCall, f: &mut SpecFields) -> bool {
    f.width_cm = a.largura_cm.unwrap_or(f.width_cm);
    f.height_cm = a.altura_cm.unwrap_or(f.height_cm);
    f.dpi = a.dpi.unwrap_or(f.dpi);
    f.bleed_mm = a.sangria_mm.unwrap_or(f.bleed_mm);
    f.safe_mm = a.zona_segura_mm.unwrap_or(f.safe_mm);
    f.spine_mm = a.lombada_mm.or(f.spine_mm);
    f.starts_single = a.inicia_com_pagina.unwrap_or(f.starts_single);
    f.ends_single = a.termina_com_pagina.unwrap_or(f.ends_single);
    let asked = [
        a.largura_cm,
        a.altura_cm,
        a.sangria_mm,
        a.zona_segura_mm,
        a.lombada_mm,
    ];
    asked.iter().any(Option::is_some)
        || a.dpi.is_some()
        || a.inicia_com_pagina.is_some()
        || a.termina_com_pagina.is_some()
}
