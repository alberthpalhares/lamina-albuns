//! Registro de comandos (spec JP, regra 10; spec TL, seção 3): cada ação tem um nome estável, um
//! rótulo e no máximo um atalho, e é chamada igual pelo menu, pelo teclado e pelos testes.

use crate::keys::Shortcut;

mod all;
mod modes;
mod table;
mod table_edit;
mod table_organize;
mod table_photo;
mod table_window;

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
    /// Endireitar a foto pelas teclas, no modo recorte (spec AF).
    TurnLeft,
    TurnRight,
    TurnLeftMore,
    TurnRightMore,
    /// O painel da direita (Alt+4) e "Ajustar a foto…" (spec AF, regras 10 e 11).
    TogglePanel,
    AdjustPhoto,
    /// A janela nova (Rodada 0b): registrados, mas inertes até a spec de cada um ligar.
    /// A faixa de layouts (FL), os atalhos e as novidades (AT e NV), a lupa e o tamanho real (ZR),
    /// montar o álbum (BR) e selecionar as fotos mostradas (PF).
    ToggleLayoutBand,
    ShowShortcuts,
    ShowNews,
    Loupe,
    ActualSize,
    CalibrateSize,
    BuildAlbum,
    SelectAllPhotos,
    /// Trocar o nome do álbum (spec NA): a janela abre o campo no topo.
    RenameAlbum,
    /// Esconder ou mostrar as três faixas de baixo de uma vez (spec FX, regra 8).
    ToggleStrips,
}

/// Nome, rótulo, menu e atalho de um comando.
type Info = (&'static str, &'static str, Menu, Option<Shortcut>);

impl Command {
    /// Nome, rótulo, menu e atalho: cada comando está numa das cinco partes da tabela (o teste
    /// `todo_comando_esta_numa_parte_da_tabela` confere).
    fn info(self) -> Info {
        self.app_info()
            .or_else(|| self.edit_info())
            .or_else(|| self.organize_info())
            .or_else(|| self.photo_info())
            .or_else(|| self.window_info())
            .unwrap_or(("", "", Menu::Help, None))
    }

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
