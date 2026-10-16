//! O editor: a sessão aberta (ou nenhuma), o estado da tela que não vai para o arquivo (guias,
//! lista de layouts ativa), as preferências de layout e a pergunta antes de sair com alterações
//! não salvas (spec JP, regras 7, 8 e 10; spec TL).

use std::cell::RefCell;
use std::collections::HashMap;
use std::path::Path;

use lamina_core::{PhotoId, Project};
use lamina_layout::{LayoutLists, Preferences};

use crate::photos::{Filter, photo_list};
use crate::prefs_store::PrefsStore;
use crate::suggestions::Part;
use crate::{Command, EditorError, Session};

mod frames;
mod layouts;

pub use layouts::Panel;

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

    /// O comando pode rodar agora? (menu e teclado obedecem; regra 10 do JP)
    pub fn available(&self, command: Command) -> bool {
        let Some(s) = &self.session else {
            return matches!(command, Command::NewAlbum | Command::Open | Command::About);
        };
        let last = s.spread_count().saturating_sub(1);
        match command {
            Command::NewAlbum | Command::Open | Command::Save | Command::SaveAs => true,
            Command::ToggleGuides | Command::About | Command::ToggleReordered => true,
            Command::NextSpread | Command::LastSpread => s.current() < last,
            Command::PreviousSpread | Command::FirstSpread => s.current() > 0,
            Command::Undo => s.can_undo(),
            Command::Redo => s.can_redo(),
            Command::NextLayout | Command::PreviousLayout => self.can_step(Part::Spread),
            Command::NextLeft | Command::PreviousLeft => self.can_step(Part::Left),
            Command::NextRight | Command::PreviousRight => self.can_step(Part::Right),
            Command::Mirror => self.can_mirror(),
            Command::Favorite | Command::Reject => self.panel(Part::Spread).in_use.is_some(),
            Command::ClearFrame => self.can_clear(),
            Command::Deselect => self.selected_frame.is_some() || !self.selection.is_empty(),
            Command::BuildFromSelection | Command::NewSpreadFromSelection => {
                !self.selection.is_empty() && self.photo_selection().is_ok()
            }
            Command::NewSpread => true,
            Command::DeleteSpread => s.spread_count() > 1,
            Command::MoveSpreadLeft => s.current() > 0,
            Command::MoveSpreadRight => s.current() < last,
        }
    }

    /// Roda os comandos que não precisam de diálogo. `true` = algo mudou. Os de arquivo pedem um
    /// caminho à janela e chamam [`Editor::open`] e a sessão.
    pub fn run(&mut self, command: Command) -> bool {
        if !self.available(command) {
            return false;
        }
        match command {
            Command::ToggleGuides => {
                self.guides = !self.guides;
                true
            }
            Command::ToggleReordered => {
                self.reordered = !self.reordered;
                true
            }
            Command::Undo => self.session.as_mut().and_then(Session::undo).is_some(),
            Command::Redo => self.session.as_mut().and_then(Session::redo).is_some(),
            Command::NextLayout => self.step(Part::Spread, true),
            Command::PreviousLayout => self.step(Part::Spread, false),
            Command::NextLeft => self.step(Part::Left, true),
            Command::PreviousLeft => self.step(Part::Left, false),
            Command::NextRight => self.step(Part::Right, true),
            Command::PreviousRight => self.step(Part::Right, false),
            Command::Mirror => self.mirror(),
            Command::Favorite => self.favorite(),
            Command::Reject => self.reject(),
            Command::ClearFrame => self.clear_selected(),
            Command::Deselect => {
                self.selected_frame = None;
                self.selection.clear();
                true
            }
            Command::BuildFromSelection => self.build_from_selection(0),
            Command::NewSpreadFromSelection => self.new_spread_from_selection(),
            Command::NewSpread => self.new_spread(),
            Command::DeleteSpread => self.delete_current(),
            Command::MoveSpreadLeft => self.move_by(-1),
            Command::MoveSpreadRight => self.move_by(1),
            _ => {
                self.selected_frame = None;
                self.navigate(command)
            }
        }
    }

    fn navigate(&mut self, command: Command) -> bool {
        let Some(s) = self.session.as_mut() else {
            return false;
        };
        let target = match command {
            Command::NextSpread => s.current() + 1,
            Command::PreviousSpread => s.current().saturating_sub(1),
            Command::FirstSpread => 0,
            Command::LastSpread => s.spread_count().saturating_sub(1),
            _ => return false,
        };
        s.go_to(target)
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
