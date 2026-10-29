//! As fontes embutidas (spec TX, seção 3; ADR 0008, adendo): seis famílias com licença SIL OFL
//! 1.1, cada uma com o `OFL.txt` na pasta. Elas vêm antes das fontes do computador (spec FS,
//! regra 2), que só entram no banco de uma linha de execução quando um texto usa uma delas.

use std::cell::RefCell;
use std::collections::HashSet;
use std::sync::Arc;

use parley::fontique::{Blob, Collection, CollectionOptions, FontInfoOverride, SourceCache};
use parley::{FontContext, LayoutContext};

use super::installed::{InstalledFont, STATE, installed_font, lock};

/// Uma família embutida: o nome que vai no projeto e o que ela tem.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Family {
    pub name: &'static str,
    pub bold: bool,
    pub italic: bool,
    regular: &'static [u8],
    italic_face: Option<&'static [u8]>,
}

impl Family {
    /// Os arquivos: o normal e o itálico, se houver (para as fontes de mentira dos testes).
    pub(crate) fn files(&self) -> (&'static [u8], Option<&'static [u8]>) {
        (self.regular, self.italic_face)
    }

    /// Os bytes do arquivo usado para checar os caracteres (o itálico, quando pedido e existe).
    pub(crate) fn face(&self, italic: bool) -> &'static [u8] {
        match self.italic_face {
            Some(face) if italic => face,
            _ => self.regular,
        }
    }
}

macro_rules! font {
    ($path:literal) => {
        include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/fonts/", $path))
    };
}

/// As famílias, na ordem da lista da janela. A primeira é a padrão (`DEFAULT_FONT`).
pub const FAMILIES: [Family; 6] = [
    Family {
        name: "Inter",
        bold: true,
        italic: true,
        regular: font!("inter/Inter.ttf"),
        italic_face: Some(font!("inter/Inter-Italic.ttf")),
    },
    Family {
        name: "Montserrat",
        bold: true,
        italic: true,
        regular: font!("montserrat/Montserrat.ttf"),
        italic_face: Some(font!("montserrat/Montserrat-Italic.ttf")),
    },
    Family {
        name: "EB Garamond",
        bold: true,
        italic: true,
        regular: font!("eb-garamond/EBGaramond.ttf"),
        italic_face: Some(font!("eb-garamond/EBGaramond-Italic.ttf")),
    },
    Family {
        name: "Playfair Display",
        bold: true,
        italic: true,
        regular: font!("playfair-display/PlayfairDisplay.ttf"),
        italic_face: Some(font!("playfair-display/PlayfairDisplay-Italic.ttf")),
    },
    Family {
        name: "Cormorant Garamond",
        bold: true,
        italic: true,
        regular: font!("cormorant-garamond/CormorantGaramond.ttf"),
        italic_face: Some(font!("cormorant-garamond/CormorantGaramond-Italic.ttf")),
    },
    Family {
        name: "Great Vibes",
        bold: false,
        italic: false,
        regular: font!("great-vibes/GreatVibes.ttf"),
        italic_face: None,
    },
];

/// A família pelo nome guardado no projeto.
pub fn family(name: &str) -> Option<&'static Family> {
    FAMILIES.iter().find(|f| f.name == name)
}

/// A família que desenha um texto: uma embutida ou uma fonte do computador.
pub(crate) enum Drawn {
    Lamina(&'static Family),
    Installed(Arc<InstalledFont>),
}

impl Drawn {
    pub(crate) fn name(&self) -> &str {
        match self {
            Drawn::Lamina(f) => f.name,
            Drawn::Installed(f) => &f.name,
        }
    }

    pub(crate) fn bold(&self) -> bool {
        match self {
            Drawn::Lamina(f) => f.bold,
            Drawn::Installed(f) => f.bold,
        }
    }

    pub(crate) fn italic(&self) -> bool {
        match self {
            Drawn::Lamina(f) => f.italic,
            Drawn::Installed(f) => f.italic,
        }
    }

    /// Os bytes do arquivo usado para checar os caracteres e a posição da face nele.
    pub(crate) fn face(&self, italic: bool) -> (&[u8], u32) {
        match self {
            Drawn::Lamina(f) => (f.face(italic), 0),
            Drawn::Installed(f) => f.face(italic),
        }
    }
}

/// A família de um nome: a embutida primeiro (spec FS, regra 2), depois a do computador. `None`
/// quando não está em nenhum dos dois. As embutidas nunca tocam nas do computador (FS-B13).
pub(crate) fn resolve(name: &str) -> Option<Drawn> {
    match family(name) {
        Some(f) => Some(Drawn::Lamina(f)),
        None => installed_font(name).map(Drawn::Installed),
    }
}

/// A família que desenha um nome: ela mesma ou, se não existir, a padrão (spec TX, seção 6).
pub(crate) fn drawn_or_default(name: &str) -> Drawn {
    resolve(name).unwrap_or(Drawn::Lamina(&FAMILIES[0]))
}

/// O banco de fontes de uma linha de execução: as embutidas; as do computador entram depois,
/// uma família por vez, quando um texto pede ([`register`]).
fn font_context() -> FontContext {
    let mut collection = Collection::new(CollectionOptions {
        shared: false,
        system_fonts: false,
    });
    for f in &FAMILIES {
        for bytes in std::iter::once(f.regular).chain(f.italic_face) {
            let data: Arc<dyn AsRef<[u8]> + Send + Sync> = Arc::new(bytes);
            collection.register_fonts(Blob::new(data), None);
        }
    }
    FontContext {
        collection,
        source_cache: SourceCache::default(),
    }
}

/// O banco de uma linha de execução e as famílias do computador já registradas nele.
struct Context {
    fonts: FontContext,
    layout: LayoutContext<[u8; 4]>,
    generation: u64,
    registered: HashSet<String>,
}

thread_local! {
    // Montar o banco custa: cada linha de execução (as da exportação em paralelo também) monta
    // uma vez e reaproveita.
    static CONTEXT: RefCell<Context> = RefCell::new(Context {
        fonts: font_context(),
        layout: LayoutContext::new(),
        generation: 0,
        registered: HashSet::new(),
    });
}

/// Põe as faces de uma família do computador no banco desta linha, com o nome da lista. Os
/// arquivos são os mesmos para todas as linhas: nada é lido de novo (FS-B15).
fn register(c: &mut Context, font: &InstalledFont) {
    for face in font.faces() {
        let name = FontInfoOverride {
            family_name: Some(&font.name),
            ..FontInfoOverride::default()
        };
        c.fonts
            .collection
            .register_fonts(face.data.clone(), Some(name));
    }
    c.registered.insert(font.name.clone());
}

/// Roda `f` com o banco de fontes (já com a família `drawn`) e o rascunho de linhas desta linha
/// de execução.
pub(crate) fn with_context<R>(
    drawn: &Drawn,
    f: impl FnOnce(&mut FontContext, &mut LayoutContext<[u8; 4]>) -> R,
) -> R {
    CONTEXT.with(|c| {
        let mut c = c.borrow_mut();
        let generation = lock(&STATE).generation;
        if c.generation != generation {
            // A fonte das fontes mudou (testes): as do computador registradas não valem mais.
            c.fonts = font_context();
            c.registered.clear();
            c.generation = generation;
        }
        if let Drawn::Installed(font) = drawn
            && !c.registered.contains(&font.name)
        {
            register(&mut c, font);
        }
        let Context { fonts, layout, .. } = &mut *c;
        f(fonts, layout)
    })
}

/// Os nomes das famílias que o banco desta linha conhece (para o teste da regra 2 da TX e o FS-B5).
pub fn known_families() -> Vec<String> {
    with_context(&Drawn::Lamina(&FAMILIES[0]), |fonts, _| {
        let mut names: Vec<String> = fonts.collection.family_names().map(str::to_owned).collect();
        names.sort();
        names
    })
}
