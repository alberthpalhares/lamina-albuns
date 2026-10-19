//! Frases dos erros do plano, dos layouts e do Auto Build (`lamina-layout`).

use lamina_layout::{BuildError, LayoutError, PlanError};

use super::{Human, HumanMessage, name, technical};

/// Para o que não serve numa foto: o editor da foto é quem resolve.
const REEXPORT: &str = "Exporte a foto de novo pelo seu editor ou tire-a da pasta.";

impl Human for LayoutError {
    fn human(&self) -> HumanMessage {
        let (title, what) = match self {
            LayoutError::NoPhotos => (
                "A lâmina não tem fotos.".to_owned(),
                "Ponha fotos na lâmina antes de escolher um layout.".to_owned(),
            ),
            LayoutError::TooManyPhotos {
                photos,
                pages,
                capacity,
            } => (
                format!("São {photos} fotos para {pages} páginas, e cabem no máximo {capacity}."),
                "Tire algumas fotos desta lâmina ou divida-as em duas lâminas.".to_owned(),
            ),
            LayoutError::InvalidAspect(i) => (
                format!("A foto {i} desta lâmina tem um tamanho que o Lamina não entende."),
                REEXPORT.to_owned(),
            ),
            LayoutError::GapTooLarge => (
                "O espaço entre as fotos não cabe na página.".to_owned(),
                "Diminua o espaço entre fotos na aba Aparência.".to_owned(),
            ),
        };
        HumanMessage::new(title, what, technical(self))
    }
}

impl Human for PlanError {
    fn human(&self) -> HumanMessage {
        let (title, what) = match self {
            PlanError::NoPhotos => (
                "Não há fotos para montar o álbum.".to_owned(),
                "Escolha uma pasta com fotos em JPEG, PNG ou TIFF.".to_owned(),
            ),
            PlanError::NoSpreads => (
                "O álbum precisa de pelo menos 1 lâmina.".to_owned(),
                "Aumente o número de lâminas.".to_owned(),
            ),
            PlanError::InvalidMaxPerPage { max, .. } => (
                format!("Fotos por página precisa ficar entre 1 e {max}."),
                "Corrija o campo Fotos por página.".to_owned(),
            ),
            PlanError::MoreSpreadsThanPhotos { photos, spreads } => (
                format!("São {spreads} lâminas para só {photos} fotos."),
                format!("Use no máximo {photos} lâminas."),
            ),
            PlanError::TooManyPhotos {
                photos,
                capacity,
                spreads,
                min_spreads,
            } => (
                format!("São {photos} fotos e cabem {capacity} em {spreads} lâminas."),
                format!("Aumente para pelo menos {min_spreads} lâminas ou tire algumas fotos."),
            ),
        };
        HumanMessage::new(title, what, technical(self))
    }
}

impl Human for BuildError {
    fn human(&self) -> HumanMessage {
        match self {
            BuildError::InvalidPhoto(photo) => HumanMessage::new(
                format!("A foto \"{}\" não tem um tamanho válido.", name(photo)),
                REEXPORT,
                technical(self),
            ),
            BuildError::Plan(e) => e.human(),
            BuildError::Layout(e) => e.human(),
            BuildError::Core(e) => e.human(),
        }
    }
}
