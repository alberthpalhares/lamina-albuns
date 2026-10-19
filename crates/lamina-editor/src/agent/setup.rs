//! O agente e o editor com a configuração do fotógrafo, os mesmos da janela: favoritos e
//! rejeitados, modelos e presets da pasta de configuração, e o cache do sistema.

use std::path::PathBuf;

use super::{Agent, AgentOptions};
use crate::{
    Cache, Editor, MODELS_FILE, ModelsStore, PREFS_FILE, PRESETS_FILE, PrefsStore, PresetsStore,
    config_dir,
};

/// O par que o servidor MCP usa: `roots` são as pastas de `--permitir`; `images` é `false` com
/// `--sem-imagens`.
pub fn photographer(roots: Vec<PathBuf>, images: bool) -> (Agent, Editor) {
    let dir = config_dir();
    let mut editor = Editor::with_prefs(PrefsStore::load(&dir.join(PREFS_FILE)));
    editor.set_models(ModelsStore::load(&dir.join(MODELS_FILE)));
    let presets = PresetsStore::load(&dir.join(PRESETS_FILE)).all();
    let options = AgentOptions {
        roots,
        images,
        cache: Cache::system(),
        presets,
    };
    (Agent::new(options), editor)
}
