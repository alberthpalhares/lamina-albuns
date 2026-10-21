//! O estado da janela: o editor, as imagens já convertidas para o Slint e o que está pendente
//! (pergunta antes de sair, trabalho em segundo plano). `refresh_*` copia o editor para a tela.

use std::collections::HashMap;
use std::path::PathBuf;

use lamina_core::{PhotoId, Project};
use lamina_editor::forms::photo_text;
use lamina_editor::motion::{photo_rows, strip_rows};
use lamina_editor::{
    Cache, Editor, ExportCheck, Filter, PhotoItem, Session, ThumbList, guarded, photo_info,
    photo_list, prepare_images,
};
use slint::{ComponentHandle, Image};

use crate::AppWindow;
use crate::convert::{photo_data, spread_data};
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
    /// O álbum que o sistema entregou (duplo clique; spec MA, regra 4).
    OpenReceived,
}

pub use crate::msg::Msg;

pub struct App {
    pub window: slint::Weak<AppWindow>,
    pub editor: Editor,
    pub cache: Cache,
    pub tx: Mailbox,
    pub pending: Option<Pending>,
    /// Pasta e número de fotos da janela "Novo álbum" aberta.
    pub new_folder: PathBuf,
    pub new_photos: usize,
    /// As etapas do Novo projeto abertas e o arquivo que o "Criar" grava (spec NP).
    pub wizard: Option<lamina_editor::new_project::NewProject>,
    pub new_target: Option<PathBuf>,
    /// Arquivos soltos na tela inicial, que o Novo projeto importa no lugar de uma pasta.
    pub new_files: Vec<PathBuf>,
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
    /// A faixa, a linha do tempo e o navegador, atualizados linha a linha (spec MV, regra 3).
    pub(crate) lists: crate::lists::Lists,
    /// As vagas da lâmina grande (spec MV, regra 4).
    pub(crate) slot_rows: crate::slots::SlotRows,
    /// Decide se ir para outra lâmina desliza (spec MV, seção 4).
    pub(crate) navigator: lamina_editor::motion::Navigator,
    /// Os recentes que a última conferência não achou e a busca da tela inicial (spec TI).
    pub recent_missing: Vec<PathBuf>,
    pub recent_query: String,
    /// Quando começou o teste de fumaça da importação (`--verificar-importacao`, spec TR).
    pub verify_import: Option<std::time::Instant>,
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
            wizard: None,
            new_target: None,
            new_files: Vec::new(),
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
            lists: crate::lists::Lists::new(window),
            slot_rows: crate::slots::SlotRows::new(),
            navigator: lamina_editor::motion::Navigator::default(),
            recent_missing: Vec::new(),
            recent_query: String::new(),
            verify_import: None,
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
        self.lists.reset();
        self.refresh_all();
    }

    /// Fotos novas no mesmo álbum (importar): as imagens que a janela já tem ficam, as listas
    /// ganham só as linhas novas e o trabalhador lê só as miniaturas novas (spec TR, regra 8).
    pub fn photos_added(&mut self) {
        self.notice = self
            .session()
            .and_then(Session::missing_message)
            .unwrap_or_default();
        self.refresh_all();
    }

    /// Imagens geradas no cache: as que faltavam são pedidas de novo e as miniaturas entram.
    pub fn images_ready(&mut self) {
        if let Some(project) = self.session().map(|s| s.project().clone()) {
            self.images.recheck(&project);
        }
        self.lists.reset();
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
        self.images.all_thumbs(project)
    }

    /// Faixa de lâminas, com as miniaturas e o marcador de aviso: só as lâminas que mudaram.
    pub fn refresh_strip(&mut self) {
        let Some(session) = self.editor.session() else {
            return;
        };
        let rows = strip_rows(session);
        let warned: Vec<bool> = rows.iter().map(|(_, r)| r.warned).collect();
        let (project, missing) = (session.project().clone(), session.missing().to_vec());
        let images = self.images.all_thumbs(&project);
        self.lists.strip.sync(rows, |i| {
            let warning = warned.get(i).copied().unwrap_or(false);
            strip_data(&project, i, &missing, &images, warning)
        });
        crate::drops::refresh_timeline(self);
    }

    /// Navegador de fotos e dados da foto selecionada: só as fotos que mudaram.
    pub fn refresh_photos(&mut self) {
        let Some(w) = self.window.upgrade() else {
            return;
        };
        let Some(project) = self.session().map(|s| s.project().clone()) else {
            return;
        };
        self.list = photo_list(&project, self.filter);
        let rows = photo_rows(&self.list, self.editor.selection());
        let picked: Vec<bool> = rows.iter().map(|(_, r)| r.picked).collect();
        let ids: Vec<PhotoId> = self.list.iter().map(|p| p.id).collect();
        let thumbs = self.images.thumbs_for(ThumbList::Browser, &project, &ids);
        let list = &self.list;
        self.lists.browser.sync(rows, |i| {
            let item = &list[i];
            let picked = picked.get(i).copied().unwrap_or(false);
            photo_data(item, thumbs.get(&item.id).cloned(), picked)
        });
        w.set_filter(match self.filter {
            Filter::All => 0,
            Filter::Used => 1,
            Filter::Unused => 2,
        });
        let selected = self
            .selected
            .and_then(|id| self.list.iter().position(|p| p.id == id));
        w.set_selected_photo(selected.and_then(|i| i32::try_from(i).ok()).unwrap_or(-1));
        let info = self.selected.and_then(|id| photo_info(&project, id));
        w.set_info_photo(info.map(|i| photo_text(&i)).unwrap_or_default().into());
    }
}

/// A linha da lâmina `i` na faixa, com as miniaturas `images` e o marcador de aviso.
pub(crate) fn strip_data(
    project: &Project,
    i: usize,
    missing: &[PhotoId],
    images: &HashMap<PhotoId, Image>,
    warning: bool,
) -> crate::SpreadData {
    let image = |id: PhotoId| images.get(&id).cloned();
    let mut data = spread_data(project, i, missing, &image, (warning, SMALL));
    // A miniatura mostra a foto ajustada (spec AF, regra 7).
    crate::adjusted::patch_small(project, i, images, &mut data);
    data
}
