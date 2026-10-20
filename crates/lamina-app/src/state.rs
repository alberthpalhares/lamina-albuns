//! O estado da janela: o editor, as imagens já convertidas para o Slint e o que está pendente
//! (pergunta antes de sair, trabalho em segundo plano). `refresh_*` copia o editor para a tela.

use std::collections::HashMap;
use std::path::PathBuf;

use lamina_core::{PhotoId, Project};
use lamina_editor::forms::photo_text;
use lamina_editor::{
    Cache, Editor, ExportCheck, Filter, Found, HumanMessage, PhotoItem, Session, Step, guarded,
    photo_info, photo_list, prepare_images,
};
use slint::{ComponentHandle, Image};

use crate::AppWindow;
use crate::convert::{model, photo_data, spread_data};
use crate::images::Images;
use crate::mailbox::Mailbox;
use crate::texts::SMALL;

/// O que fazer depois da pergunta "Salvar as alterações?".
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pending {
    Close,
    Open,
    NewAlbum,
    /// "Reiniciar e atualizar" (spec DS, regra 5).
    Update,
    /// O álbum de exemplo (spec PC, regra 4).
    Example,
    /// Fechar o álbum e voltar à tela inicial; abrir o recente da posição dada (spec TI).
    CloseAlbum,
    OpenRecent(usize),
}

/// Mensagens do trabalho em segundo plano para a linha da janela.
#[derive(Debug)]
pub enum Msg {
    Step(Step),
    Album(Result<(Project, usize), HumanMessage>),
    ImagesReady,
    /// Uma foto de tela lida pelo trabalhador (spec MV, regra 1); `None` sem imagem de tela.
    Screen {
        epoch: u64,
        id: PhotoId,
        pixels: Option<crate::images::Pixels>,
    },
    /// A foto original do quadro em modo recorte (spec RC, regra 9), já em pixels da janela.
    Original(PhotoId, crate::images::Pixels),
    /// Andamento e fim da exportação (spec EI): o resumo ou o erro, e a pasta.
    Exporting(usize, usize),
    Exported(Result<String, HumanMessage>, PathBuf),
    /// Andamento e resultado da procura de fotos ausentes (spec EI). `file` é o projeto em que a
    /// procura começou; `folder`, a pasta escolhida (nenhuma = a procura ao abrir).
    Searching(usize, usize),
    Relinked {
        file: Option<PathBuf>,
        found: Vec<Found>,
        folder: Option<PathBuf>,
    },
    /// A resposta de uma checagem ou download da atualização (spec DS).
    Update(lamina_editor::updates::Outcome),
    /// Uma tarefa em segundo plano entrou em pânico (spec EH, regra 6): a janela se libera.
    Failed(HumanMessage),
    /// Os recentes cujo arquivo sumiu e a miniatura gravada (spec TI, regras 4 e 5).
    RecentsChecked(Vec<PathBuf>),
    ThumbReady,
}

pub struct App {
    pub window: slint::Weak<AppWindow>,
    pub editor: Editor,
    pub cache: Cache,
    pub tx: Mailbox,
    pub pending: Option<Pending>,
    /// Pasta e número de fotos da janela "Novo álbum" aberta.
    pub new_folder: PathBuf,
    pub new_photos: usize,
    pub notice: String,
    /// A conferência da janela Exportar aberta (avisos que sobraram e arquivos que já existem).
    pub export_check: Option<ExportCheck>,
    /// Presets (spec PR) e o que eles trazem para o "Novo álbum" e a "Especificação do álbum".
    pub presets: lamina_editor::PresetsStore,
    pub new_export: lamina_core::ExportPrefs,
    pub spec_export: lamina_core::ExportPrefs,
    /// Para onde vai o preset salvo (0 = novo álbum, 1 = especificação) e se o nome repetido
    /// já foi confirmado.
    pub preset_target: i32,
    pub name_guard: lamina_editor::naming::ReplaceGuard,
    /// A atualização automática (spec DS): o estado e o instalador.
    pub updates: lamina_editor::Updates,
    pub updater: crate::updates::Shared,
    pub(crate) images: Images,
    /// Foto original sendo carregada para o modo recorte.
    pub(crate) loading: Option<PhotoId>,
    /// Fecha o gesto de zoom da roda depois de uma pausa (spec RC, regra 4).
    pub(crate) crop_timer: slint::Timer,
    /// A medição de `LAMINA_TEMPOS=1` (spec MV, seção 4).
    pub(crate) clock: crate::timings::Clock,
    /// Os recentes que a última conferência não achou e a busca da tela inicial (spec TI).
    pub recent_missing: Vec<PathBuf>,
    pub recent_query: String,
    filter: Filter,
    list: Vec<PhotoItem>,
    selected: Option<PhotoId>,
}

impl App {
    pub fn new(window: &AppWindow, cache: Cache, editor: Editor, tx: Mailbox) -> Self {
        let images = Images::new(&cache, &tx);
        App {
            window: window.as_weak(),
            editor,
            cache,
            tx,
            pending: None,
            new_folder: PathBuf::new(),
            new_photos: 0,
            notice: String::new(),
            export_check: None,
            presets: lamina_editor::PresetsStore::default(),
            new_export: lamina_core::ExportPrefs::default(),
            spec_export: lamina_core::ExportPrefs::default(),
            preset_target: 0,
            name_guard: lamina_editor::naming::ReplaceGuard::default(),
            updates: lamina_editor::Updates::new(
                lamina_editor::UpdatesStore::default(),
                lamina_editor::updates::Version::default(),
                false,
            ),
            updater: crate::updates::Shared::default(),
            images,
            loading: None,
            crop_timer: crate::crop::timer(),
            clock: crate::timings::Clock::default(),
            recent_missing: Vec::new(),
            recent_query: String::new(),
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
                // Uma falha inesperada vai para o registro; a janela segue com o que tem.
                let _ = guarded(|| prepare_images(&project, 0, &cache));
                // Se a janela já fechou, não há a quem avisar.
                let _ = tx.send(Msg::ImagesReady);
            });
        }
        self.images.clear();
        self.refresh_all();
    }

    /// Imagens geradas no cache: as que faltavam são pedidas de novo e as miniaturas entram.
    pub fn images_ready(&mut self) {
        self.images.recheck();
        self.refresh_all();
    }

    pub fn set_filter(&mut self, filter: Filter) {
        self.filter = filter;
        self.refresh_photos();
    }

    /// As fotos na ordem em que o navegador mostra agora.
    pub fn shown_photos(&self) -> Vec<PhotoId> {
        self.list.iter().map(|p| p.id).collect()
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
            .map(|(i, warning)| spread_data(&project, i, &missing, &image, (*warning, SMALL)))
            .collect();
        w.set_spreads(model(spreads));
        crate::drag::refresh_timeline(self);
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
            .map(|item| {
                let picked = self.editor.selection().contains(&item.id);
                photo_data(
                    item,
                    self.images.thumb(&project, item.id, &self.cache),
                    picked,
                )
            })
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
