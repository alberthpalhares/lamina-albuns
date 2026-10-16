//! O editor: a sessão aberta (ou nenhuma), o estado da tela que não vai para o arquivo (guias,
//! lista de layouts ativa), as preferências de layout e a pergunta antes de sair com alterações
//! não salvas (spec JP, regras 7, 8 e 10; spec TL).

use std::cell::RefCell;
use std::collections::HashMap;
use std::path::Path;

use lamina_core::{Crop, PhotoId, Project};
use lamina_layout::{LayoutLists, Preferences};

use crate::photos::{Filter, photo_list};
use crate::prefs_store::PrefsStore;
use crate::{EditorError, Session};

mod crop;
mod dispatch;
mod frames;
mod layouts;
mod style;

pub use crop::ZOOM_STEP;
pub use layouts::Panel;
pub use style::Look;

/// Resposta à pergunta "Salvar as alterações?".
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Leave {
    Save,
    DontSave,
    Cancel,
}

/// Listas do motor já calculadas, pelas proporções das fotos e páginas ativas (regra 10 do TL).
type ListsKey = (Vec<u64>, bool, bool);

#[derive(Debug)]
pub struct Editor {
    session: Option<Session>,
    guides: bool,
    reordered: bool,
    prefs: PrefsStore,
    lists: RefCell<HashMap<ListsKey, LayoutLists>>,
    notice: Option<String>,
    selected_frame: Option<usize>,
    selection: Vec<PhotoId>,
    /// Última foto clicada sem Shift: começo do intervalo do Shift+clique.
    anchor: Option<PhotoId>,
    /// Quadro em modo recorte e o recorte provisório do gesto (spec RC).
    crop_frame: Option<usize>,
    pending_crop: Option<Crop>,
}

impl Default for Editor {
    fn default() -> Self {
        Editor::new()
    }
}

impl Editor {
    /// Editor com as preferências só na memória (testes e linha de comando).
    pub fn new() -> Self {
        Editor::with_prefs(PrefsStore::default())
    }

    /// Editor com as preferências do arquivo do fotógrafo. O aviso de leitura, se houver, vira
    /// o primeiro aviso da janela.
    pub fn with_prefs(mut prefs: PrefsStore) -> Self {
        let notice = prefs.warning.take();
        Editor {
            session: None,
            guides: true,
            reordered: false,
            prefs,
            lists: RefCell::new(HashMap::new()),
            notice,
            selected_frame: None,
            selection: Vec::new(),
            anchor: None,
            crop_frame: None,
            pending_crop: None,
        }
    }

    pub fn session(&self) -> Option<&Session> {
        self.session.as_ref()
    }

    pub fn session_mut(&mut self) -> Option<&mut Session> {
        self.session.as_mut()
    }

    pub fn guides(&self) -> bool {
        self.guides
    }

    /// A lista ativa no painel é a das reordenadas?
    pub fn reordered(&self) -> bool {
        self.reordered
    }

    pub fn preferences(&self) -> &Preferences {
        &self.prefs.prefs
    }

    /// Aviso para a janela mostrar (uma vez): preferências que não leram ou não gravaram.
    pub fn take_notice(&mut self) -> Option<String> {
        self.notice.take()
    }

    /// Abre o arquivo. Se não der, a sessão anterior continua como estava (JP-B11).
    pub fn open(&mut self, file: &Path) -> Result<(), EditorError> {
        self.session = Some(Session::open(file)?);
        self.selected_frame = None;
        self.selection.clear();
        Ok(())
    }

    /// Passa a editar um álbum novo, ainda sem arquivo.
    pub fn adopt(&mut self, project: Project) {
        self.session = Some(Session::new_unsaved(project));
        self.selected_frame = None;
        self.selection.clear();
    }

    /// Fechar, abrir outro ou criar outro precisa perguntar antes?
    pub fn needs_confirmation(&self) -> bool {
        self.session.as_ref().is_some_and(Session::is_dirty)
    }

    /// Aplica a resposta à pergunta. `true` = pode seguir (fechar, abrir, criar). Salvar um
    /// projeto sem arquivo usa `save_to`, escolhido no diálogo "Salvar como".
    pub fn leave(&mut self, choice: Leave, save_to: Option<&Path>) -> Result<bool, EditorError> {
        match choice {
            Leave::Cancel => Ok(false),
            Leave::DontSave => Ok(true),
            Leave::Save => {
                let session = self.session.as_mut().ok_or(EditorError::NoProject)?;
                match (session.file(), save_to) {
                    (_, Some(target)) => session.save_as(target)?,
                    (Some(_), None) => session.save()?,
                    (None, None) => return Err(EditorError::NoFile),
                };
                Ok(true)
            }
        }
    }

    /// Vai para a primeira lâmina da foto (duplo clique no navegador). `false` se não é usada.
    pub fn go_to_photo(&mut self, id: PhotoId) -> bool {
        let Some(s) = self.session.as_mut() else {
            return false;
        };
        let first = photo_list(s.project(), Filter::All)
            .into_iter()
            .find(|item| item.id == id)
            .and_then(|item| item.first_spread);
        first.is_some_and(|index| s.go_to(index))
    }
}
