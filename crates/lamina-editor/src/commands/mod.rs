//! Registro de comandos (spec JP, regra 10; spec TL, seção 3): cada ação tem um nome estável, um
//! rótulo e no máximo um atalho, e é chamada igual pelo menu, pelo teclado e pelos testes.

use crate::keys::Shortcut;

mod modes;
mod table;
mod table_edit;
mod table_organize;

pub use modes::Mode;

/// Menu onde o comando aparece.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Menu {
    File,
    Edit,
    Spread,
    Layout,
    View,
    Help,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Command {
    NewAlbum,
    Open,
    Save,
    SaveAs,
    Export,
    FindMissing,
    Specification,
    Presets,
    Undo,
    Redo,
    NextSpread,
    PreviousSpread,
    FirstSpread,
    LastSpread,
    NextLayout,
    PreviousLayout,
    NextLeft,
    PreviousLeft,
    NextRight,
    PreviousRight,
    Mirror,
    Favorite,
    Reject,
    ToggleReordered,
    ClearFrame,
    Deselect,
    BuildFromSelection,
    NewSpreadFromSelection,
    NewSpread,
    DeleteSpread,
    MoveSpreadLeft,
    MoveSpreadRight,
    CropMode,
    ZoomIn,
    ZoomOut,
    ResetCrop,
    ToggleGuides,
    ToggleSpreads,
    ToggleTimeline,
    TogglePhotos,
    About,
    NewText,
    NewPhotoFrame,
    BringForward,
    SendBackward,
    SaveModel,
    Models,
    CheckUpdates,
    ToggleAutoUpdates,
    ToggleBeta,
    SendFeedback,
    OpenLogs,
    OpenExample,
    /// Fechar o álbum e voltar à tela inicial (spec TI, regra 6).
    CloseAlbum,
    ClearRecents,
    /// A etapa 2 do Novo projeto para o álbum aberto (spec NP, regra 11).
    MountAlbum,
    /// Movimento sem animação (spec MV, regra 14).
    ToggleReduceMotion,
    /// Fotos a qualquer momento (spec IM): arquivos ou uma pasta.
    ImportPhotos,
    ImportFolder,
    /// O modo organizar a lâmina (spec EG) e o que vale dentro dele.
    Organize,
    SelectAllFrames,
    NudgeLeft,
    NudgeRight,
    NudgeUp,
    NudgeDown,
    NudgeLeftMore,
    NudgeRightMore,
    NudgeUpMore,
    NudgeDownMore,
    ViewZoomIn,
    ViewZoomOut,
    ViewFit,
    /// Alinhar, Igualar e Distribuir (spec EG, regra 9).
    AlignLeft,
    AlignCenter,
    AlignRight,
    AlignTop,
    AlignMiddle,
    AlignBottom,
    EqualWidth,
    EqualHeight,
    EqualSize,
    DistributeAcross,
    DistributeDown,
    /// A área de cópia (spec EG, regra 12).
    CopyFrames,
    PasteFrames,
    DuplicateFrames,
}

/// Nome, rótulo, menu e atalho de um comando.
type Info = (&'static str, &'static str, Menu, Option<Shortcut>);

impl Command {
    /// Nome, rótulo, menu e atalho: cada comando está numa das três partes da tabela (o teste
    /// `todo_comando_esta_numa_parte_da_tabela` confere).
    fn info(self) -> Info {
        self.app_info()
            .or_else(|| self.edit_info())
            .or_else(|| self.organize_info())
            .unwrap_or(("", "", Menu::Help, None))
    }

    pub const ALL: [Command; 86] = [
        Command::NewAlbum,
        Command::Open,
        Command::Save,
        Command::SaveAs,
        Command::Export,
        Command::FindMissing,
        Command::Specification,
        Command::Presets,
        Command::Undo,
        Command::Redo,
        Command::NextSpread,
        Command::PreviousSpread,
        Command::FirstSpread,
        Command::LastSpread,
        Command::NextLayout,
        Command::PreviousLayout,
        Command::NextLeft,
        Command::PreviousLeft,
        Command::NextRight,
        Command::PreviousRight,
        Command::Mirror,
        Command::Favorite,
        Command::Reject,
        Command::ToggleReordered,
        Command::ClearFrame,
        Command::Deselect,
        Command::BuildFromSelection,
        Command::NewSpreadFromSelection,
        Command::NewSpread,
        Command::DeleteSpread,
        Command::MoveSpreadLeft,
        Command::MoveSpreadRight,
        Command::CropMode,
        Command::ZoomIn,
        Command::ZoomOut,
        Command::ResetCrop,
        Command::ToggleGuides,
        Command::ToggleSpreads,
        Command::ToggleTimeline,
        Command::TogglePhotos,
        Command::About,
        Command::NewText,
        Command::NewPhotoFrame,
        Command::BringForward,
        Command::SendBackward,
        Command::SaveModel,
        Command::Models,
        Command::CheckUpdates,
        Command::ToggleAutoUpdates,
        Command::ToggleBeta,
        Command::SendFeedback,
        Command::OpenLogs,
        Command::OpenExample,
        Command::CloseAlbum,
        Command::ClearRecents,
        Command::MountAlbum,
        Command::ToggleReduceMotion,
        Command::ImportPhotos,
        Command::ImportFolder,
        Command::Organize,
        Command::SelectAllFrames,
        Command::NudgeLeft,
        Command::NudgeRight,
        Command::NudgeUp,
        Command::NudgeDown,
        Command::NudgeLeftMore,
        Command::NudgeRightMore,
        Command::NudgeUpMore,
        Command::NudgeDownMore,
        Command::ViewZoomIn,
        Command::ViewZoomOut,
        Command::ViewFit,
        Command::AlignLeft,
        Command::AlignCenter,
        Command::AlignRight,
        Command::AlignTop,
        Command::AlignMiddle,
        Command::AlignBottom,
        Command::EqualWidth,
        Command::EqualHeight,
        Command::EqualSize,
        Command::DistributeAcross,
        Command::DistributeDown,
        Command::CopyFrames,
        Command::PasteFrames,
        Command::DuplicateFrames,
    ];

    /// Nome estável, para testes e para a linha de comando.
    pub fn name(self) -> &'static str {
        self.info().0
    }

    /// Rótulo do menu (e do histórico, nas edições).
    pub fn label(self) -> &'static str {
        self.info().1
    }

    pub fn menu(self) -> Menu {
        self.info().2
    }

    pub fn shortcut(self) -> Option<Shortcut> {
        self.info().3
    }

    /// O comando de nome `name`, se houver.
    pub fn from_name(name: &str) -> Option<Command> {
        Command::ALL.into_iter().find(|c| c.name() == name)
    }
}
