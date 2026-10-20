//! # lamina-editor
//!
//! A lógica do aplicativo, sem janela (spec `janela-principal`; regra 1 e ADR 0001). A janela
//! Slint (`lamina-app`) só desenha estes dados e chama estes comandos; a linha de comando usa o
//! mesmo caminho para montar um álbum novo.
//!
//! - [`album`]: novo álbum a partir de uma pasta (importar → sugerir lâminas → Auto Build).
//! - [`cache`]: catálogo, miniaturas e imagens de tela no cache do sistema.
//! - [`session`]: o projeto aberto, o arquivo dele, a lâmina atual e as alterações não salvas.
//! - [`editor`]: a sessão (ou nenhuma), os comandos e a pergunta antes de sair.
//! - [`commands`] e [`keys`]: o registro de comandos, com nome, rótulo e atalho.
//! - [`suggestions`]: as sugestões de layout da lâmina, trocar e espelhar (spec TL).
//! - [`prefs_store`]: favoritos e rejeitados no arquivo de preferências do fotógrafo.
//! - [`text`]: nome das famílias e nota explicada, para o painel de layouts.
//! - [`view`]: a lâmina em pixels da tela, igual à impressão.
//! - [`photos`]: o navegador de fotos.
//! - [`presets`] e [`respec`]: presets e trocar a especificação de um álbum pronto (spec PR).
//! - [`strips`]: as três faixas de baixo, com título e recolhíveis (spec PN).
//! - [`recents`]: a lista de recentes e o que a tela inicial mostra (spec TI).
//! - [`forms`]: campos, passos e textos das janelas (novo álbum, exportar, aparência).
//! - [`exporting`]: exportar pela janela, com a pré-impressão antes (spec EI).
//! - [`relink`]: reencontrar fotos que mudaram de pasta (spec EI).
//! - [`screens`]: as imagens de tela na memória, só da lâmina atual e das vizinhas.
//! - [`agent`]: o controle por IA, as ferramentas que o servidor MCP oferece (spec IA).
//! - [`human`]: cada erro dito ao fotógrafo, com o que fazer e o detalhe (spec EH).
//! - [`error_log`] e [`feedback`]: o registro de erros e o formulário de sugestões (spec SR).

pub mod agent;
pub mod album;
pub mod appearance;
mod applying;
pub mod cache;
pub mod commands;
pub mod editor;
pub mod edits;
pub mod error_log;
pub mod example;
mod export_form;
pub mod exporting;
pub mod feedback;
pub mod forms;
pub mod frame_form;
pub mod hand;
pub mod human;
pub mod keys;
mod look_form;
pub mod models;
pub mod naming;
pub mod new_project;
pub mod photos;
pub mod prefs_store;
pub mod presets;
pub mod recents;
pub mod relink;
pub mod respec;
pub mod screens;
pub mod session;
pub mod snap;
pub mod spreads;
pub mod strips;
pub mod suggestions;
pub mod text;
pub mod updates;
pub mod view;

mod error;
#[cfg(test)]
mod tests;

pub use album::{
    Imported, NewAlbum, SpecFields, Step, check_spreads, count_photos, create_album, folder_name,
    import_project, project_from, suggestion, suggestion_for, suggestion_text, suggestion_within,
};
pub use cache::Cache;
pub use commands::{Command, Menu};
pub use editor::{Editor, Leave, Panel, TextChange};
pub use error::EditorError;
pub use error_log::ErrorLog;
pub use example::{EXAMPLE_COPY, EXAMPLE_DIR, EXAMPLE_FILE, example_target, prepare_example};
pub use exporting::{
    DEFAULT_QUALITY, ExportCheck, ExportFormat, ExportOptions, Warning, export_progress,
    export_project, export_summary,
};
pub use feedback::{FEEDBACK_FORM, feedback_address};
pub use human::{Human, HumanMessage, guarded};
pub use keys::{Key, Shortcut};
pub use lamina_render::text_image;
pub use models::{MODELS_FILE, ModelError, ModelsStore, model_from_spread};
pub use photos::{Filter, PhotoInfo, PhotoItem, photo_info, photo_list, used_photos};
pub use prefs_store::{PREFS_FILE, PrefsStore, config_dir};
pub use presets::{PRESETS_FILE, Preset, PresetError, PresetsStore, builtin_presets};
pub use recents::{RECENTS_FILE, Recent, RecentsStore};
pub use relink::{
    Found, RELINK_LATER, not_found_message, relinked_message, search_folder, search_near,
};
pub use screens::{ScreenImages, load_original, prepare_images};
pub use session::{HistoryList, HistoryStep, Session};
pub use snap::{Grip, SnapLine};
pub use strips::{STRIPS_FILE, STRIPS_UNREADABLE, Strip, Strips, StripsStore, strip_titles};
pub use suggestions::{List, Part, Unavailable};
pub use text::{
    BAD_COLOR, PALETTE, color_hex, family_name, frame_dpi_text, option_title, parse_color,
    score_text,
};
pub use updates::{UPDATES_FILE, Updates, UpdatesStore};
pub use view::{
    Area, FrameKind, FrameView, PxRect, SpreadView, frame_at, photo_extent, spread_view,
};
