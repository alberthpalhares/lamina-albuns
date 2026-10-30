//! De onde vêm as fontes do computador (spec FS, regra 1): o índice do sistema, pelo `fontique`
//! (DirectWrite no Windows, Core Text no Mac, fontconfig no Linux), ou uma lista dos testes.

use parley::fontique::{Blob, Collection, CollectionOptions, FontInfo, FontStyle, SourceCache};

/// Uma face de uma família instalada: o arquivo (mapeado, não copiado), a posição dele numa
/// coleção `.ttc` e o que ela tem.
#[derive(Debug, Clone)]
pub struct Face {
    pub data: Blob<u8>,
    pub index: u32,
    pub bold: bool,
    pub italic: bool,
}

/// Onde o Lamina procura as fontes do computador.
pub trait FontSource: Send {
    /// Os nomes de todas as famílias, pelo índice do sistema, sem abrir arquivos.
    fn names(&mut self) -> Vec<String>;
    /// As faces de uma família, abrindo só os arquivos dela.
    fn faces(&mut self, name: &str) -> Vec<Face>;
}

/// O índice do sistema. Ele só é criado no primeiro pedido (regra 11): abrir o Lamina não lê
/// nada.
#[derive(Default)]
pub(crate) struct SystemSource {
    collection: Option<Collection>,
    cache: SourceCache,
}

impl SystemSource {
    fn collection(&mut self) -> &mut Collection {
        self.collection.get_or_insert_with(|| {
            Collection::new(CollectionOptions {
                shared: false,
                system_fonts: true,
            })
        })
    }
}

impl FontSource for SystemSource {
    fn names(&mut self) -> Vec<String> {
        self.collection()
            .family_names()
            .map(str::to_owned)
            .collect()
    }

    fn faces(&mut self, name: &str) -> Vec<Face> {
        let Some(family) = self.collection().family_by_name(name) else {
            return Vec::new();
        };
        family
            .fonts()
            .iter()
            .filter_map(|font| {
                Some(Face {
                    data: font.load(Some(&mut self.cache))?,
                    index: font.index(),
                    bold: is_bold(font),
                    italic: is_italic(font),
                })
            })
            .collect()
    }
}

/// Negrito: um arquivo de peso 600 ou mais, ou uma fonte variável que chega lá (regra 3).
fn is_bold(font: &FontInfo) -> bool {
    font.weight().value() >= 600.0
        || font
            .axes()
            .iter()
            .any(|a| a.tag.to_be_bytes() == *b"wght" && a.max >= 600.0)
}

/// Itálico: um arquivo itálico ou oblíquo, ou uma fonte variável com esse eixo (regra 3).
fn is_italic(font: &FontInfo) -> bool {
    font.style() != FontStyle::Normal || font.has_italic_axis() || font.has_slant_axis()
}
