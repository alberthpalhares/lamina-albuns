//! As dicas da paleta do modo organizar (spec ZR, regra 1): uma linha por botão, com o nome, o que
//! ele faz e a tecla tirada do registro de comandos (nunca escrita à mão), e " (indisponível)"
//! quando não vale agora.

use crate::editor::PaletteTool;
use crate::{Command, Editor};

/// Quanto tempo o mouse fica parado sobre o botão antes do balão aparecer.
pub const TIP_DELAY_MS: u32 = 500;

/// Os botões da paleta, na ordem da tela (a janela recebe as dicas nesta ordem).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PaletteButton {
    Tool(PaletteTool),
    /// Um botão que roda um comando do registro (zoom, 1:1, Ajustar, Alinhar…).
    Run(Command),
}

/// Todos os botões, na ordem da paleta.
pub const PALETTE_BUTTONS: [PaletteButton; 20] = [
    PaletteButton::Tool(PaletteTool::Select),
    PaletteButton::Tool(PaletteTool::Frame),
    PaletteButton::Tool(PaletteTool::Text),
    PaletteButton::Tool(PaletteTool::Hand),
    PaletteButton::Tool(PaletteTool::Magnifier),
    PaletteButton::Run(Command::ViewZoomOut),
    PaletteButton::Run(Command::ViewZoomIn),
    PaletteButton::Run(Command::ActualSize),
    PaletteButton::Run(Command::ViewFit),
    PaletteButton::Run(Command::AlignLeft),
    PaletteButton::Run(Command::AlignCenter),
    PaletteButton::Run(Command::AlignRight),
    PaletteButton::Run(Command::AlignTop),
    PaletteButton::Run(Command::AlignMiddle),
    PaletteButton::Run(Command::AlignBottom),
    PaletteButton::Run(Command::EqualWidth),
    PaletteButton::Run(Command::EqualHeight),
    PaletteButton::Run(Command::EqualSize),
    PaletteButton::Run(Command::DistributeAcross),
    PaletteButton::Run(Command::DistributeDown),
];

/// A tecla do comando como a dica mostra ("Z", "+", "−", "1").
fn key(command: Command) -> String {
    command
        .shortcut()
        .map(|s| s.text().replace('-', "−"))
        .unwrap_or_default()
}

/// O texto do botão, sem o "(indisponível)".
fn text(button: PaletteButton) -> String {
    use PaletteTool as T;
    match button {
        PaletteButton::Tool(T::Select) => "Selecionar e mover quadros".to_owned(),
        PaletteButton::Tool(T::Frame) => "Quadro de foto: arraste para desenhar".to_owned(),
        PaletteButton::Tool(T::Text) => "Texto: arraste para desenhar".to_owned(),
        PaletteButton::Tool(T::Hand) => {
            "Mão: arraste para mover a vista (ou segure Espaço)".to_owned()
        }
        PaletteButton::Tool(T::Magnifier) => format!(
            "Lupa ({}): clique aproxima, Alt+clique afasta",
            key(Command::Loupe)
        ),
        PaletteButton::Run(Command::ViewZoomOut) => {
            format!("Afastar ({})", key(Command::ViewZoomOut))
        }
        PaletteButton::Run(Command::ViewZoomIn) => {
            format!("Aproximar ({})", key(Command::ViewZoomIn))
        }
        PaletteButton::Run(Command::ActualSize) => {
            format!("Tamanho real ({})", key(Command::ActualSize))
        }
        PaletteButton::Run(Command::ViewFit) => {
            format!("Ajustar a lâmina à área ({})", key(Command::ViewFit))
        }
        PaletteButton::Run(Command::DistributeAcross) => {
            "Distribuir na horizontal (três quadros ou mais)".to_owned()
        }
        PaletteButton::Run(Command::DistributeDown) => {
            "Distribuir na vertical (três quadros ou mais)".to_owned()
        }
        PaletteButton::Run(command) => command.label().to_owned(),
    }
}

impl Editor {
    /// A dica de um botão da paleta agora: com " (indisponível)" quando não vale.
    pub fn palette_tip(&self, button: PaletteButton) -> String {
        let available = match button {
            PaletteButton::Tool(_) => self.organizing(),
            PaletteButton::Run(command) => self.available(command),
        };
        let text = text(button);
        if available {
            text
        } else {
            format!("{text} (indisponível)")
        }
    }

    /// As dicas de todos os botões, na ordem de [`PALETTE_BUTTONS`]. Com o 1:1 estimado, a dica
    /// dele diz como calibrar (regra 9).
    pub fn palette_tips(&self) -> Vec<String> {
        PALETTE_BUTTONS
            .iter()
            .map(|b| {
                let tip = self.palette_tip(*b);
                if *b == PaletteButton::Run(Command::ActualSize) && !self.calibrated() {
                    format!("{tip}. {}", crate::calibration::ESTIMATE_TIP)
                } else {
                    tip
                }
            })
            .collect()
    }
}
