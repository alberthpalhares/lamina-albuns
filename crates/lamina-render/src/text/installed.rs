//! As fontes do computador (spec FS): o que já foi lido, guardado até o programa fechar, e quantas
//! vezes cada coisa foi lida (regra 11). As 6 embutidas nunca passam por aqui.

use std::collections::HashMap;
use std::sync::{Arc, LazyLock, Mutex, MutexGuard, PoisonError};

use skrifa::{FontRef, MetadataProvider};

use super::fonts::family;
use super::source::{Face, FontSource, SystemSource};

/// Uma família instalada, já aberta (o termo "fonte do computador" do glossário).
#[derive(Debug)]
pub struct InstalledFont {
    pub name: String,
    pub bold: bool,
    pub italic: bool,
    faces: Vec<Face>,
}

impl InstalledFont {
    pub(crate) fn faces(&self) -> &[Face] {
        &self.faces
    }

    /// A face usada para checar os caracteres (os bytes e a posição dela no arquivo): a itálica,
    /// quando pedida e existe.
    pub(crate) fn face(&self, italic: bool) -> (&[u8], u32) {
        let upright = self.faces.iter().find(|f| !f.italic && !f.bold);
        let wanted = if italic {
            self.faces.iter().find(|f| f.italic)
        } else {
            upright
        };
        wanted
            .or(upright)
            .or(self.faces.first())
            .map_or((&[], 0), |f| (f.data.as_ref(), f.index))
    }
}

/// Quantas vezes as fontes do computador foram lidas: a lista dos nomes e os arquivos abertos.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct FontReads {
    pub names: u32,
    pub files: u32,
}

#[derive(Default)]
pub(crate) struct State {
    /// Muda quando a fonte das fontes é trocada (testes): as linhas de execução refazem o banco.
    pub(crate) generation: u64,
    names: Option<Arc<Vec<String>>>,
    loading: bool,
    families: HashMap<String, Option<Arc<InstalledFont>>>,
    pub(crate) reads: FontReads,
}

pub(crate) static SOURCE: LazyLock<Mutex<Box<dyn FontSource>>> =
    LazyLock::new(|| Mutex::new(Box::new(SystemSource::default())));
pub(crate) static STATE: LazyLock<Mutex<State>> = LazyLock::new(Mutex::default);

/// Uma trava envenenada (um teste que caiu) não deve derrubar as fontes do resto.
pub(crate) fn lock<T: ?Sized>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(PoisonError::into_inner)
}

/// Os nomes das fontes do computador, em ordem alfabética, sem as que têm o nome de uma
/// embutida (regra 2). Lê o índice do sistema na primeira vez e guarda (regra 11).
pub fn installed_names() -> Arc<Vec<String>> {
    if let Some(names) = installed_names_ready() {
        return names;
    }
    let mut source = lock(&SOURCE);
    // Outra linha pode ter lido enquanto esta esperava.
    if let Some(names) = installed_names_ready() {
        return names;
    }
    let names = Arc::new(clean(source.names()));
    let mut state = lock(&STATE);
    state.names = Some(Arc::clone(&names));
    state.reads.names += 1;
    names
}

/// Os nomes, se já foram lidos; nunca espera (FS-B11).
pub fn installed_names_ready() -> Option<Arc<Vec<String>>> {
    lock(&STATE).names.clone()
}

/// Começa a ler os nomes em segundo plano, se ainda não leu nem começou (regra 1).
pub fn request_installed_names() {
    {
        let mut state = lock(&STATE);
        if state.names.is_some() || state.loading {
            return;
        }
        state.loading = true;
    }
    let read = std::thread::Builder::new()
        .name("fontes do computador".into())
        .spawn(|| {
            installed_names();
            lock(&STATE).loading = false;
        });
    if read.is_err() {
        lock(&STATE).loading = false;
    }
}

/// O nome certo de uma fonte do computador, sem diferença de maiúsculas (a IA, regra 8).
pub fn installed_name(query: &str) -> Option<String> {
    let query = query.trim();
    installed_names()
        .iter()
        .find(|n| n.to_lowercase() == query.to_lowercase())
        .cloned()
}

/// A família instalada com esse nome, aberta uma vez só. `None` para um nome embutido, que não
/// está no computador, ou cujos arquivos não abrem (seção 6).
pub(crate) fn installed_font(name: &str) -> Option<Arc<InstalledFont>> {
    if family(name).is_some() {
        return None;
    }
    if let Some(known) = lock(&STATE).families.get(name) {
        return known.clone();
    }
    let listed = installed_names().iter().any(|n| n == name);
    let mut source = lock(&SOURCE);
    if let Some(known) = lock(&STATE).families.get(name) {
        return known.clone();
    }
    let faces = if listed {
        source.faces(name)
    } else {
        Vec::new()
    };
    let opened = u32::try_from(faces.len()).unwrap_or(u32::MAX);
    let usable: Vec<Face> = faces.into_iter().filter(usable).collect();
    let font = (!usable.is_empty()).then(|| {
        Arc::new(InstalledFont {
            name: name.to_owned(),
            bold: usable.iter().any(|f| f.bold),
            italic: usable.iter().any(|f| f.italic),
            faces: usable,
        })
    });
    let mut state = lock(&STATE);
    state.reads.files += opened;
    state.families.insert(name.to_owned(), font.clone());
    font
}

/// Uma face abre e tem as letras básicas (seção 6: arquivo corrompido fica de fora).
fn usable(face: &Face) -> bool {
    FontRef::from_index(face.data.as_ref(), face.index)
        .is_ok_and(|f| f.charmap().map('A').is_some())
}

/// Tira os nomes escondidos do sistema ("@…" no Windows, "." no Mac), os vazios, os repetidos e
/// os iguais aos das embutidas, e põe em ordem alfabética.
fn clean(mut names: Vec<String>) -> Vec<String> {
    names.retain(|n| {
        !n.trim().is_empty()
            && !n.starts_with(['@', '.'])
            && !super::fonts::FAMILIES
                .iter()
                .any(|f| f.name.eq_ignore_ascii_case(n))
    });
    names.sort_by_cached_key(|n| (n.to_lowercase(), n.clone()));
    names.dedup();
    names
}

/// Troca a fonte das fontes e esquece tudo o que foi lido (para os testes).
pub(crate) fn replace_source(source: Box<dyn FontSource>) {
    let mut current = lock(&SOURCE);
    *current = source;
    let mut state = lock(&STATE);
    let generation = state.generation + 1;
    *state = State {
        generation,
        ..State::default()
    };
}
