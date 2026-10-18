//! As fontes embutidas (spec TX, seção 3; ADR 0008, adendo): seis famílias com licença SIL OFL
//! 1.1, cada uma com o `OFL.txt` na pasta. Só elas existem para o renderizador (regra 2): nenhuma
//! fonte do sistema entra, para o mesmo projeto sair igual em qualquer máquina.

use std::cell::RefCell;
use std::sync::Arc;

use parley::fontique::{Blob, Collection, CollectionOptions, SourceCache};
use parley::{FontContext, LayoutContext};

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

/// A família que desenha um nome: ele mesmo ou, se não existir, a padrão (spec TX, seção 6).
pub(crate) fn family_or_default(name: &str) -> &'static Family {
    family(name).unwrap_or(&FAMILIES[0])
}

/// O banco de fontes de uma linha de execução: só as embutidas, nunca as do sistema.
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

thread_local! {
    // Montar o banco custa: cada linha de execução (as da exportação em paralelo também) monta
    // uma vez e reaproveita.
    static CONTEXT: RefCell<(FontContext, LayoutContext<[u8; 4]>)> =
        RefCell::new((font_context(), LayoutContext::new()));
}

/// Roda `f` com o banco de fontes e o rascunho de linhas desta linha de execução.
pub(crate) fn with_context<R>(
    f: impl FnOnce(&mut FontContext, &mut LayoutContext<[u8; 4]>) -> R,
) -> R {
    CONTEXT.with(|c| {
        let mut c = c.borrow_mut();
        let (fonts, layout) = &mut *c;
        f(fonts, layout)
    })
}

/// Os nomes das famílias que o banco conhece (para o teste da regra 2).
pub fn known_families() -> Vec<String> {
    with_context(|fonts, _| {
        let mut names: Vec<String> = fonts.collection.family_names().map(str::to_owned).collect();
        names.sort();
        names
    })
}
