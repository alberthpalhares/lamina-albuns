//! A tabela do registro: nome estável, rótulo, menu e atalho de cada comando.

use super::{Command, Info, Menu};
use crate::keys::{Key, Shortcut};

impl Command {
    pub(super) fn info(self) -> Info {
        use Command as C;
        use Shortcut as S;
        let k = |key| Some(S::key(key));
        let shift = |key| Some(S::shift(key));
        match self {
            C::NewAlbum => (
                "arquivo.novo",
                "Novo álbum…",
                Menu::File,
                Some(S::ctrl('n')),
            ),
            C::Open => ("arquivo.abrir", "Abrir…", Menu::File, Some(S::ctrl('o'))),
            C::Save => ("arquivo.salvar", "Salvar", Menu::File, Some(S::ctrl('s'))),
            C::SaveAs => (
                "arquivo.salvar-como",
                "Salvar como…",
                Menu::File,
                Some(S::ctrl_shift('s')),
            ),
            C::Export => (
                "arquivo.exportar",
                "Exportar…",
                Menu::File,
                Some(S::ctrl('e')),
            ),
            C::OpenExample => (
                "arquivo.exemplo",
                "Abrir o álbum de exemplo",
                Menu::File,
                None,
            ),
            C::FindMissing => (
                "arquivo.procurar-fotos",
                "Procurar fotos ausentes…",
                Menu::File,
                None,
            ),
            C::Specification => (
                "arquivo.especificacao",
                "Especificação do álbum…",
                Menu::File,
                None,
            ),
            C::Presets => ("arquivo.presets", "Presets…", Menu::File, None),
            C::Undo => (
                "editar.desfazer",
                "Desfazer",
                Menu::Edit,
                Some(S::ctrl('z')),
            ),
            C::Redo => ("editar.refazer", "Refazer", Menu::Edit, Some(S::ctrl('y'))),
            C::NextSpread => (
                "lamina.proxima",
                "Próxima lâmina",
                Menu::Spread,
                k(Key::PageDown),
            ),
            C::PreviousSpread => (
                "lamina.anterior",
                "Lâmina anterior",
                Menu::Spread,
                k(Key::PageUp),
            ),
            C::FirstSpread => (
                "lamina.primeira",
                "Primeira lâmina",
                Menu::Spread,
                k(Key::Home),
            ),
            C::LastSpread => ("lamina.ultima", "Última lâmina", Menu::Spread, k(Key::End)),
            C::NextLayout => (
                "layout.proximo",
                "Próximo layout",
                Menu::Layout,
                k(Key::Right),
            ),
            C::PreviousLayout => (
                "layout.anterior",
                "Layout anterior",
                Menu::Layout,
                k(Key::Left),
            ),
            C::NextLeft => (
                "layout.esquerda-proximo",
                "Próximo layout da página esquerda",
                Menu::Layout,
                k(Key::Down),
            ),
            C::PreviousLeft => (
                "layout.esquerda-anterior",
                "Layout anterior da página esquerda",
                Menu::Layout,
                k(Key::Up),
            ),
            C::NextRight => (
                "layout.direita-proximo",
                "Próximo layout da página direita",
                Menu::Layout,
                shift(Key::Down),
            ),
            C::PreviousRight => (
                "layout.direita-anterior",
                "Layout anterior da página direita",
                Menu::Layout,
                shift(Key::Up),
            ),
            C::Mirror => (
                "layout.espelhar",
                "Espelhar lâmina",
                Menu::Layout,
                k(Key::Char('e')),
            ),
            C::Favorite => (
                "layout.favoritar",
                "Favoritar layout",
                Menu::Layout,
                k(Key::Char('f')),
            ),
            C::Reject => (
                "layout.rejeitar",
                "Rejeitar layout",
                Menu::Layout,
                k(Key::Char('x')),
            ),
            C::ToggleReordered => (
                "layout.reordenadas",
                "Mostrar reordenadas",
                Menu::Layout,
                k(Key::Char('r')),
            ),
            C::ClearFrame => (
                "editar.tirar-foto",
                "Tirar foto ou apagar quadro",
                Menu::Edit,
                k(Key::Delete),
            ),
            C::Deselect => ("editar.desmarcar", "Desmarcar", Menu::Edit, k(Key::Escape)),
            C::BuildFromSelection => (
                "layout.montar",
                "Montar lâmina",
                Menu::Layout,
                k(Key::Enter),
            ),
            C::NewSpreadFromSelection => (
                "lamina.nova-com-selecao",
                "Nova lâmina com a seleção",
                Menu::Spread,
                shift(Key::Enter),
            ),
            C::NewSpread => (
                "lamina.nova",
                "Nova lâmina",
                Menu::Spread,
                Some(S::ctrl_shift('n')),
            ),
            C::DeleteSpread => (
                "lamina.apagar",
                "Apagar lâmina",
                Menu::Spread,
                Some(S::ctrl_key(Key::Delete)),
            ),
            C::MoveSpreadLeft => (
                "lamina.mover-esquerda",
                "Mover lâmina para a esquerda",
                Menu::Spread,
                Some(S::ctrl_key(Key::Left)),
            ),
            C::MoveSpreadRight => (
                "lamina.mover-direita",
                "Mover lâmina para a direita",
                Menu::Spread,
                Some(S::ctrl_key(Key::Right)),
            ),
            C::CropMode => (
                "recorte.abrir",
                "Recortar foto",
                Menu::Edit,
                k(Key::Char('c')),
            ),
            C::ZoomIn => (
                "recorte.mais-zoom",
                "Mais zoom",
                Menu::Edit,
                k(Key::Char('+')),
            ),
            C::ZoomOut => (
                "recorte.menos-zoom",
                "Menos zoom",
                Menu::Edit,
                k(Key::Char('-')),
            ),
            C::ResetCrop => (
                "recorte.encaixar",
                "Encaixar a foto",
                Menu::Edit,
                k(Key::Char('0')),
            ),
            C::ToggleGuides => ("ver.guias", "Mostrar guias", Menu::View, k(Key::Char('g'))),
            C::ToggleSpreads => (
                "ver.faixa-laminas",
                "Faixa de lâminas",
                Menu::View,
                Some(S::alt('1')),
            ),
            C::ToggleTimeline => (
                "ver.linha-do-tempo",
                "Linha do tempo",
                Menu::View,
                Some(S::alt('2')),
            ),
            C::TogglePhotos => ("ver.fotos", "Fotos", Menu::View, Some(S::alt('3'))),
            C::About => ("ajuda.sobre", "Sobre o Lamina", Menu::Help, None),
            C::NewText => (
                "lamina.novo-texto",
                "Novo texto",
                Menu::Spread,
                k(Key::Char('t')),
            ),
            C::NewPhotoFrame => (
                "lamina.novo-quadro",
                "Novo quadro de foto",
                Menu::Spread,
                None,
            ),
            C::BringForward => ("lamina.frente", "Trazer para a frente", Menu::Spread, None),
            C::SendBackward => ("lamina.tras", "Enviar para trás", Menu::Spread, None),
            C::SaveModel => (
                "lamina.salvar-modelo",
                "Salvar como modelo…",
                Menu::Spread,
                None,
            ),
            C::Models => ("lamina.modelos", "Meus modelos…", Menu::Spread, None),
            C::CheckUpdates => (
                "ajuda.verificar-atualizacoes",
                "Verificar atualizações agora",
                Menu::Help,
                None,
            ),
            C::ToggleAutoUpdates => (
                "ajuda.atualizar-automaticamente",
                "Verificar atualizações automaticamente",
                Menu::Help,
                None,
            ),
            C::ToggleBeta => (
                "ajuda.versoes-de-teste",
                "Receber versões de teste (beta)",
                Menu::Help,
                None,
            ),
            C::SendFeedback => (
                "ajuda.enviar-sugestao",
                "Enviar sugestão ou problema…",
                Menu::Help,
                None,
            ),
            C::OpenLogs => (
                "ajuda.abrir-registros",
                "Abrir pasta de registros",
                Menu::Help,
                None,
            ),
        }
    }
}
