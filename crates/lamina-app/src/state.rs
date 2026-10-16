//! O estado da janela: o editor, as imagens já convertidas para o Slint e o que está pendente
//! (pergunta antes de sair, trabalho em segundo plano). `refresh_*` copia o editor para a tela.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::mpsc::Sender;

use lamina_core::{PhotoId, Project};
use lamina_editor::{
    Cache, Command, Editor, Filter, PhotoItem, Session, Step, photo_info, photo_list,
    prepare_images,
};
use slint::{ComponentHandle, Image};

use crate::AppWindow;
use crate::convert::{model, photo_data, spread_data};
use crate::images::Images;
use crate::info::{album_info, photo_text};

/// O que fazer depois da pergunta "Salvar as alterações?".
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pending {
    Close,
    Open,
    NewAlbum,
}

/// Mensagens do trabalho em segundo plano para a linha da janela.
#[derive(Debug)]
pub enum Msg {
    Step(Step),
    Album(Result<(Project, usize), String>),
    ImagesReady,
}

pub struct App {
    pub window: slint::Weak<AppWindow>,
    pub editor: Editor,
    pub cache: Cache,
    pub tx: Sender<Msg>,
    pub pending: Option<Pending>,
    /// Pasta e número de fotos da janela "Novo álbum" aberta.
    pub new_folder: PathBuf,
    pub new_photos: usize,
    pub notice: String,
    images: Images,
    filter: Filter,
    list: Vec<PhotoItem>,
    selected: Option<PhotoId>,
}

impl App {
    pub fn new(window: &AppWindow, cache: Cache, editor: Editor, tx: Sender<Msg>) -> Self {
        App {
            window: window.as_weak(),
            editor,
            cache,
            tx,
            pending: None,
            new_folder: PathBuf::new(),
            new_photos: 0,
            notice: String::new(),
            images: Images::default(),
            filter: Filter::All,
            list: Vec::new(),
            selected: None,
        }
    }

    pub fn session(&self) -> Option<&Session> {
        self.editor.session()
    }

    /// Projeto novo na tela: limpa as imagens e gera as que faltam em segundo plano.
    pub fn project_changed(&mut self) {
        self.selected = None;
        self.notice = self
            .session()
            .and_then(Session::missing_message)
            .unwrap_or_default();
        if let Some(session) = self.session() {
            let project = session.project().clone();
            let (cache, tx) = (self.cache.clone(), self.tx.clone());
            std::thread::spawn(move || {
                prepare_images(&project, 0, &cache);
                // Se a janela já fechou, não há a quem avisar.
                let _ = tx.send(Msg::ImagesReady);
            });
        }
        self.images_ready();
    }

    /// Imagens geradas (ou projeto novo): recarrega tudo do cache.
    pub fn images_ready(&mut self) {
        self.images.clear();
        self.refresh_all();
    }

    pub fn set_filter(&mut self, filter: Filter) {
        self.filter = filter;
        self.refresh_photos();
    }

    /// Foto `index` do navegador, na lista mostrada agora.
    pub fn photo_at(&self, index: i32) -> Option<PhotoId> {
        let i = usize::try_from(index).ok()?;
        self.list.get(i).map(|p| p.id)
    }

    pub fn select_photo(&mut self, id: Option<PhotoId>) {
        self.selected = id;
        self.refresh_photos();
    }

    pub fn refresh_all(&mut self) {
        self.refresh_strip();
        self.refresh_photos();
        self.refresh_current();
    }

    /// Título, contador, comandos disponíveis e a lâmina atual com o painel dela.
    pub fn refresh_current(&mut self) {
        let Some(w) = self.window.upgrade() else {
            return;
        };
        w.set_has_project(self.session().is_some());
        w.set_can_save(self.editor.available(Command::Save));
        w.set_can_prev(self.editor.available(Command::PreviousSpread));
        w.set_can_next(self.editor.available(Command::NextSpread));
        w.set_guides(self.editor.guides());
        w.set_notice(self.notice.clone().into());
        let Some(session) = self.editor.session() else {
            w.set_window_title("Lâmina".into());
            w.set_project_title("".into());
            w.set_counter("".into());
            return;
        };
        let dirty = if session.is_dirty() { "• " } else { "" };
        let title = format!("{dirty}{}", session.title());
        w.set_window_title(format!("{title} — Lâmina").into());
        w.set_project_title(title.into());
        w.set_counter(session.counter().into());
        let index = session.current();
        let warnings: Vec<String> = session
            .warnings_for(index)
            .iter()
            .map(ToString::to_string)
            .collect();
        let (project, missing) = (session.project().clone(), session.missing().to_vec());
        let count = session.spread_count();
        let frames = project.spreads.get(index).map_or(0, |s| s.frames.len());
        w.set_info_album(album_info(&project).into());
        w.set_info_spread(format!("Lâmina {} de {count} · {frames} quadros", index + 1).into());
        w.set_info_warnings(warnings.join("\n").into());

        let images = self.images.spread(&project, index, &self.cache);
        let image = |id: PhotoId| images.get(&id).cloned();
        let current = spread_data(&project, index, &missing, &image, !warnings.is_empty());
        w.set_current(current);
        w.set_current_index(i32::try_from(index).unwrap_or(0));
        if let Some(notice) = self.editor.take_notice() {
            self.notice = notice;
            w.set_notice(self.notice.clone().into());
        }
        crate::panel::refresh(self);
    }

    /// As miniaturas de todas as fotos (miniaturas do painel de layouts).
    pub fn thumbs(&mut self, project: &Project) -> HashMap<PhotoId, Image> {
        self.images.all_thumbs(project, &self.cache)
    }

    /// Faixa de lâminas, com as miniaturas e o marcador de aviso.
    pub fn refresh_strip(&mut self) {
        let Some(w) = self.window.upgrade() else {
            return;
        };
        let Some(session) = self.editor.session() else {
            return;
        };
        let (project, missing) = (session.project().clone(), session.missing().to_vec());
        let warned: Vec<bool> = (0..session.spread_count())
            .map(|i| !session.warnings_for(i).is_empty())
            .collect();
        let images = self.images.all_thumbs(&project, &self.cache);
        let image = |id: PhotoId| images.get(&id).cloned();
        let spreads = warned
            .iter()
            .enumerate()
            .map(|(i, warning)| spread_data(&project, i, &missing, &image, *warning))
            .collect();
        w.set_spreads(model(spreads));
    }

    /// Navegador de fotos e dados da foto selecionada.
    pub fn refresh_photos(&mut self) {
        let Some(w) = self.window.upgrade() else {
            return;
        };
        let Some(project) = self.session().map(|s| s.project().clone()) else {
            return;
        };
        self.list = photo_list(&project, self.filter);
        let list = self.list.clone();
        let items = list
            .iter()
            .map(|item| photo_data(item, self.images.thumb(&project, item.id, &self.cache)))
            .collect();
        w.set_photos(model(items));
        w.set_filter(match self.filter {
            Filter::All => 0,
            Filter::Used => 1,
            Filter::Unused => 2,
        });
        let selected = self
            .selected
            .and_then(|id| list.iter().position(|p| p.id == id));
        w.set_selected_photo(selected.and_then(|i| i32::try_from(i).ok()).unwrap_or(-1));
        let info = self.selected.and_then(|id| photo_info(&project, id));
        w.set_info_photo(info.map(|i| photo_text(&i)).unwrap_or_default().into());
    }
}
