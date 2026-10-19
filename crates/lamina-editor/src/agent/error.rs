//! As recusas das ferramentas da IA, com as frases da spec IA (seção 6): o que aconteceu e o que
//! fazer, sem detalhe técnico.

use thiserror::Error;

use crate::EditorError;

#[derive(Debug, Error)]
pub enum ToolError {
    #[error("Nenhum álbum aberto. Use abrir_album ou novo_album.")]
    NoAlbum,
    #[error("O álbum tem {count} lâminas; não existe a lâmina {asked}.")]
    NoSpread { asked: usize, count: usize },
    #[error("A lâmina {spread} tem {count} quadros; não existe o quadro {asked}.")]
    NoFrame {
        spread: usize,
        count: usize,
        asked: usize,
    },
    #[error("Não existe a foto {0} neste álbum. Use listar_fotos para ver os números.")]
    NoPhoto(u64),
    #[error("O quadro {frame} da lâmina {spread} é de texto. Escolha um quadro de foto.")]
    TextFrame { spread: usize, frame: usize },
    #[error(
        "A pasta {0} não está liberada para a IA. Libere-a nas configurações do Lamina no Claude \
         (ou com --permitir)."
    )]
    NotAllowed(String),
    #[error(
        "O álbum {0} tem alterações não salvas. Salve com salvar_album ou repita com descartar: true."
    )]
    Unsaved(String),
    #[error(
        "O arquivo {0} mudou no disco depois de aberto (talvez no Lamina). Salve com outro nome para \
         não perder nada."
    )]
    ChangedOnDisk(String),
    #[error("O álbum ainda não tem arquivo. Use salvar_album com caminho.")]
    NoFile,
    #[error("Já existe o arquivo {0}. Use substituir: true ou outro caminho.")]
    FileExists(String),
    #[error(
        "Já existem {count} arquivos em {folder} (por exemplo, {example}). Use substituir: true ou \
         escolha outra pasta."
    )]
    ExportExists {
        count: usize,
        folder: String,
        example: String,
    },
    #[error(
        "A pasta {folder} tem {count} arquivos de uma exportação anterior que esta não substitui \
         (por exemplo, {example}). A IA não apaga arquivos: apague-os você mesmo ou escolha outra \
         pasta."
    )]
    Leftovers {
        count: usize,
        folder: String,
        example: String,
    },
    #[error("A lâmina {0} mudou depois da lista de layouts. Peça sugerir_layouts de novo.")]
    StaleList(usize),
    #[error("Peça sugerir_layouts da lâmina {0} antes de aplicar um layout.")]
    NoList(usize),
    #[error("A lista tem {count} opções; não existe a opção {asked}.")]
    NoOption { asked: usize, count: usize },
    #[error("Não existe o comando {0}. A lista está em lamina://comandos.")]
    UnknownCommand(String),
    #[error("O comando {name} abre uma janela. Use a ferramenta {tool} no lugar.")]
    DialogCommand { name: String, tool: &'static str },
    #[error("O comando {0} só existe na janela do Lamina.")]
    WindowOnly(String),
    #[error("O comando {0} não se aplica agora a esta lâmina e a este quadro.")]
    NotNow(String),
    #[error("Não há nada para desfazer.")]
    NothingToUndo,
    #[error("Não há nada para refazer.")]
    NothingToRedo,
    #[error(
        "As imagens estão desligadas nas configurações do Lamina. Use listar_fotos para os dados \
         das fotos."
    )]
    ImagesOff,
    #[error("Peça até {0} fotos por vez.")]
    TooManyPhotos(usize),
    #[error("Escolha pelo menos uma foto.")]
    NoPhotosChosen,
    #[error("A foto {0} aparece duas vezes na lista.")]
    RepeatedPhoto(u64),
    #[error("Escolha dois quadros diferentes.")]
    SameFrame,
    #[error("A posição vai de 1 a {0}.")]
    BadPosition(usize),
    #[error("O álbum precisa de pelo menos uma lâmina.")]
    LastSpread,
    #[error("Não existe o preset {name}. Os presets são: {known}.")]
    NoPreset { name: String, known: String },
    /// Uma parte da lâmina que não pode trocar de layout, ou outra recusa do editor já em
    /// português (spec TL, seção 6).
    #[error("{0}")]
    Refused(String),
    #[error("Não existe a ferramenta {0}.")]
    UnknownTool(String),
    /// Parâmetros que não casam com o esquema: viram erro de parâmetro do MCP (regra 13).
    #[error("Parâmetros inválidos: {0}")]
    BadArguments(String),
    #[error(transparent)]
    Editor(#[from] EditorError),
    #[error(transparent)]
    Export(#[from] lamina_export::ExportError),
    #[error(transparent)]
    Core(#[from] lamina_core::CoreError),
}
