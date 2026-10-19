//! O catálogo das ferramentas (spec IA, seção 3): nome, descrição e esquema dos parâmetros de
//! cada uma, numa fonte só. O servidor MCP e o manifesto do pacote do Claude Desktop saem daqui.

use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::Value;

use super::ToolError;
use super::calls::{
    ApplyLayout, BuildSpread, ExportCall, FrameArg, ListPhotos, MoveSpread, NewAlbumCall, NoArgs,
    OpenAlbum, PlacePhoto, Position, RunCommand, SaveAlbum, SpreadArg, Steps, SuggestLayouts,
    SwapPhotos, ViewPhotos, ViewSpread,
};

/// Uma chamada de ferramenta, já com os parâmetros conferidos.
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "ferramenta", content = "parametros")]
pub enum ToolCall {
    #[serde(rename = "abrir_album")]
    OpenAlbum(OpenAlbum),
    #[serde(rename = "novo_album")]
    NewAlbum(NewAlbumCall),
    #[serde(rename = "salvar_album")]
    SaveAlbum(SaveAlbum),
    #[serde(rename = "resumo_do_album")]
    Summary(NoArgs),
    #[serde(rename = "ver_lamina")]
    ViewSpread(ViewSpread),
    #[serde(rename = "listar_fotos")]
    ListPhotos(ListPhotos),
    #[serde(rename = "ver_fotos")]
    ViewPhotos(ViewPhotos),
    #[serde(rename = "pre_impressao")]
    Preflight(NoArgs),
    #[serde(rename = "sugerir_layouts")]
    SuggestLayouts(SuggestLayouts),
    #[serde(rename = "aplicar_layout")]
    ApplyLayout(ApplyLayout),
    #[serde(rename = "espelhar")]
    Mirror(SpreadArg),
    #[serde(rename = "colocar_foto")]
    PlacePhoto(PlacePhoto),
    #[serde(rename = "trocar_fotos")]
    SwapPhotos(SwapPhotos),
    #[serde(rename = "tirar_foto")]
    ClearFrame(FrameArg),
    #[serde(rename = "montar_lamina")]
    BuildSpread(BuildSpread),
    #[serde(rename = "nova_lamina")]
    NewSpread(Position),
    #[serde(rename = "apagar_lamina")]
    DeleteSpread(SpreadArg),
    #[serde(rename = "mover_lamina")]
    MoveSpread(MoveSpread),
    #[serde(rename = "desfazer")]
    Undo(Steps),
    #[serde(rename = "refazer")]
    Redo(Steps),
    #[serde(rename = "historico")]
    History(NoArgs),
    #[serde(rename = "exportar")]
    Export(ExportCall),
    #[serde(rename = "executar_comando")]
    RunCommand(RunCommand),
}

impl ToolCall {
    /// A chamada da ferramenta `name` com os `args` do cliente (`null` = sem parâmetros).
    pub fn parse(name: &str, args: Value) -> Result<ToolCall, ToolError> {
        if !tools().iter().any(|t| t.name == name) {
            return Err(ToolError::UnknownTool(name.to_owned()));
        }
        let args = if args.is_null() {
            Value::Object(serde_json::Map::new())
        } else {
            args
        };
        let call = serde_json::json!({ "ferramenta": name, "parametros": args });
        serde_json::from_value(call).map_err(|e| ToolError::BadArguments(e.to_string()))
    }
}

/// Uma ferramenta do catálogo.
#[derive(Debug, Clone)]
pub struct ToolInfo {
    pub name: &'static str,
    pub description: &'static str,
    /// Esquema JSON dos parâmetros.
    pub schema: Value,
}

fn info<T: JsonSchema>(name: &'static str, description: &'static str) -> ToolInfo {
    ToolInfo {
        name,
        description,
        schema: schemars::schema_for!(T).to_value(),
    }
}

/// Todas as ferramentas, na ordem da spec.
pub fn tools() -> Vec<ToolInfo> {
    vec![
        info::<OpenAlbum>(
            "abrir_album",
            "Abre um álbum (.lamina) do Lamina, mesmo com fotos faltando.",
        ),
        info::<NewAlbumCall>(
            "novo_album",
            "Cria um álbum a partir de uma pasta de fotos: importa, sugere as lâminas, monta pelo \
             Auto Build e grava o .lamina, como o Novo álbum da janela.",
        ),
        info::<SaveAlbum>(
            "salvar_album",
            "Grava o álbum no arquivo dele ou, com caminho, salva como outro arquivo. O Lamina não \
             salva sozinho.",
        ),
        info::<NoArgs>(
            "resumo_do_album",
            "Resumo do álbum aberto: especificação, lâminas, fotos usadas e sem uso, avisos, \
             histórico, alterações não salvas e os presets disponíveis.",
        ),
        info::<ViewSpread>(
            "ver_lamina",
            "Uma lâmina: quadros numerados com página, posição em mm a partir do corte, foto, DPI \
             efetivo, recorte e texto, os avisos e a prévia em imagem.",
        ),
        info::<ListPhotos>(
            "listar_fotos",
            "As fotos do álbum em ordem de captura, com número, nome, data, câmera, tamanho e as \
             lâminas onde aparecem.",
        ),
        info::<ViewPhotos>(
            "ver_fotos",
            "As miniaturas de até 24 fotos, para escolher destaques e protagonistas.",
        ),
        info::<NoArgs>(
            "pre_impressao",
            "Todos os avisos de pré-impressão (foto ampliada, quadro vazio, perto do corte, texto \
             fora da zona segura), com lâmina e quadro.",
        ),
        info::<SuggestLayouts>(
            "sugerir_layouts",
            "As opções de layout de uma lâmina (ou de uma página), com a nota de 0 a 100 e as \
             partes dela, favorita e em uso.",
        ),
        info::<ApplyLayout>(
            "aplicar_layout",
            "Aplica uma opção da última lista de sugerir_layouts da mesma lâmina e parte.",
        ),
        info::<SpreadArg>(
            "espelhar",
            "Espelha a lâmina: a esquerda troca com a direita.",
        ),
        info::<PlacePhoto>("colocar_foto", "Põe uma foto num quadro de foto."),
        info::<SwapPhotos>(
            "trocar_fotos",
            "Troca as fotos de dois quadros da mesma lâmina.",
        ),
        info::<FrameArg>("tirar_foto", "Esvazia um quadro de foto."),
        info::<BuildSpread>(
            "montar_lamina",
            "Cria uma lâmina nova com as fotos escolhidas, no primeiro layout da lista.",
        ),
        info::<Position>("nova_lamina", "Cria uma lâmina vazia."),
        info::<SpreadArg>(
            "apagar_lamina",
            "Apaga uma lâmina; as fotos dela voltam a ficar sem uso.",
        ),
        info::<MoveSpread>("mover_lamina", "Muda uma lâmina de lugar."),
        info::<Steps>("desfazer", "Desfaz passos do histórico, como Ctrl+Z."),
        info::<Steps>("refazer", "Refaz passos desfeitos, como Ctrl+Y."),
        info::<NoArgs>(
            "historico",
            "Os passos do histórico, com rótulo, o atual e o que está salvo.",
        ),
        info::<ExportCall>(
            "exportar",
            "Exporta para a gráfica em JPEG, PDF/X-4 ou TIFF, conferindo antes. Nunca substitui \
             arquivos sem substituir: true, e nunca apaga arquivos.",
        ),
        info::<RunCommand>(
            "executar_comando",
            "Roda um comando sem diálogo pelo nome dos menus (setas de layout, favoritar, \
             rejeitar, frente e trás...) numa lâmina e num quadro.",
        ),
    ]
}
