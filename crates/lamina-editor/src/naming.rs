//! Nomes dados pelo fotógrafo (presets, spec PR; modelos, spec MD) e os textos que a janela mostra
//! sobre eles e sobre a lâmina (auditoria do Marco 3, M4: a decisão e o texto saem do app).

/// O item da lista de presets para quem não usa nenhum.
pub const CUSTOM_PRESET: &str = "Personalizado";

/// O que se está nomeando: muda só a mensagem.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Named {
    Preset,
    Model,
}

/// Lembra o nome repetido que o fotógrafo já viu no aviso: o segundo clique em Salvar com o
/// **mesmo** nome substitui; trocar o nome pede confirmação de novo (PR, regra 2; MD, regra 1).
#[derive(Debug, Clone, Default)]
pub struct ReplaceGuard {
    confirmed: Option<String>,
}

impl ReplaceGuard {
    /// Abriu o diálogo: nada confirmado ainda.
    pub fn reset(&mut self) {
        self.confirmed = None;
    }

    /// Pode salvar `name`? `Err` traz o aviso para a janela quando o nome já existe e ainda não
    /// foi confirmado.
    pub fn check(&mut self, what: Named, name: &str, exists: bool) -> Result<(), String> {
        let name = name.trim();
        if !exists || self.confirmed.as_deref() == Some(name) {
            return Ok(());
        }
        self.confirmed = Some(name.to_owned());
        let kind = match what {
            Named::Preset => "um preset",
            Named::Model => "um modelo",
        };
        Err(format!(
            "Já existe {kind} \"{name}\". Clique em Salvar de novo para substituir."
        ))
    }
}

/// Aviso depois de salvar o modelo.
pub fn model_saved(name: &str) -> String {
    format!("Modelo \"{}\" salvo.", name.trim())
}

/// A linha da lâmina na aba Informações.
pub fn spread_line(index: usize, count: usize, frames: usize, manual: bool) -> String {
    let by_hand = if manual { " · montada à mão" } else { "" };
    format!(
        "Lâmina {} de {count} · {frames} quadros{by_hand}",
        index + 1
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nome_repetido_pede_confirmacao_so_para_o_mesmo_nome() {
        let mut guard = ReplaceGuard::default();
        assert_eq!(guard.check(Named::Preset, "Novo", false), Ok(()));
        let first = guard.check(Named::Preset, "A", true);
        assert_eq!(
            first,
            Err("Já existe um preset \"A\". Clique em Salvar de novo para substituir.".into())
        );
        // Trocou para outro nome que também existe: pergunta de novo.
        assert_eq!(
            guard.check(Named::Preset, "B", true),
            Err("Já existe um preset \"B\". Clique em Salvar de novo para substituir.".into())
        );
        assert_eq!(guard.check(Named::Preset, " B ", true), Ok(()));
        guard.reset();
        assert!(
            guard
                .check(Named::Model, "B", true)
                .is_err_and(|m| m.contains("um modelo"))
        );
    }

    #[test]
    fn textos_da_lamina_e_do_modelo() {
        assert_eq!(spread_line(0, 4, 3, false), "Lâmina 1 de 4 · 3 quadros");
        assert_eq!(
            spread_line(2, 4, 5, true),
            "Lâmina 3 de 4 · 5 quadros · montada à mão"
        );
        assert_eq!(model_saved(" Abertura "), "Modelo \"Abertura\" salvo.");
    }
}
