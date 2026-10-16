//! Identificadores de fotos, lâminas e quadros.

use serde::{Deserialize, Serialize};

macro_rules! id_type {
    ($(#[$doc:meta])* $name:ident) => {
        $(#[$doc])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
        #[serde(transparent)]
        pub struct $name(pub(crate) u64);

        impl $name {
            pub fn get(self) -> u64 {
                self.0
            }
        }
    };
}

id_type!(
    /// Identificador de uma foto dentro do projeto.
    PhotoId
);
id_type!(
    /// Identificador de uma lâmina dentro do projeto.
    SpreadId
);
id_type!(
    /// Identificador de um quadro dentro do projeto.
    FrameId
);
