//! Comandos do registro pela IA (decisão 3 da spec IA): rodar um comando sem diálogo pelo nome
//! estável, numa lâmina e num quadro. Os de diálogo apontam a ferramenta que faz o mesmo.

use super::{Agent, RunCommand, ToolError, ToolReply, frame_index, session, spread_index};
use crate::{Command, Editor};

impl Agent {
    /// Um comando do registro pelo nome, numa lâmina e num quadro (decisão 3 da spec).
    pub(super) fn run_command(
        &mut self,
        editor: &mut Editor,
        a: &RunCommand,
    ) -> Result<ToolReply, ToolError> {
        let command = Command::from_name(&a.comando)
            .ok_or_else(|| ToolError::UnknownCommand(a.comando.clone()))?;
        if is_dialog(command) {
            let name = a.comando.clone();
            return Err(match tool_for(command) {
                Some(tool) => ToolError::DialogCommand { name, tool },
                None => ToolError::WindowOnly(name),
            });
        }
        if let Some(lamina) = a.lamina {
            let index = spread_index(session(editor)?.project(), lamina)?;
            editor.go_to_spread(index);
        }
        if let Some(quadro) = a.quadro {
            let s = session(editor)?;
            let frame = frame_index(s.project(), s.current(), quadro)?;
            editor.select_frame(Some(frame));
        }
        if !editor.available(command) {
            return Err(ToolError::NotNow(a.comando.clone()));
        }
        editor.run(command);
        let index = session(editor)?.current();
        self.edited(
            editor,
            index,
            &format!("{} na lâmina {}.", command.label(), index + 1),
        )
    }
}

/// Os comandos que abrem uma janela de diálogo: a IA usa uma ferramenta no lugar.
pub(super) fn is_dialog(command: Command) -> bool {
    use Command as C;
    matches!(
        command,
        C::NewAlbum
            | C::Open
            | C::Save
            | C::SaveAs
            | C::Export
            | C::FindMissing
            | C::Specification
            | C::Presets
            | C::About
            | C::SaveModel
            | C::Models
            | C::CheckUpdates
            | C::ToggleAutoUpdates
            | C::ToggleBeta
    )
}

/// A ferramenta que faz o mesmo que o comando de diálogo, se houver.
fn tool_for(command: Command) -> Option<&'static str> {
    match command {
        Command::NewAlbum => Some("novo_album"),
        Command::Open => Some("abrir_album"),
        Command::Save | Command::SaveAs => Some("salvar_album"),
        Command::Export => Some("exportar"),
        _ => None,
    }
}
