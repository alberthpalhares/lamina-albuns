//! O servidor MCP: as ferramentas, os prompts prontos e os recursos, sobre o agente do
//! `lamina-editor`. A ferramenta roda fora da linha do protocolo, uma de cada vez, e o andamento
//! vira notificação de progresso (spec IA, regra 10).

use std::sync::{Arc, Mutex};

use lamina_editor::Editor;
use lamina_editor::agent::{Agent, ToolCall, ToolError, ToolReply, photographer, tools};
use rmcp::model::{
    CallToolRequestParams, CallToolResponse, GetPromptRequestParams, GetPromptResponse,
    Implementation, ListPromptsResult, ListResourceTemplatesResult, ListResourcesResult,
    ListToolsResult, PaginatedRequestParams, ProgressNotificationParam, ProgressToken,
    ReadResourceRequestParams, ReadResourceResponse, ServerCapabilities, ServerConfig,
};
use rmcp::service::RequestContext;
use rmcp::{ErrorData, RoleServer, ServerHandler};
use serde_json::Value;

use crate::{McpError, ServeOptions, convert, prompts, resources};

/// O que o cliente lê ao ligar (seção 3).
const INSTRUCTIONS: &str = "Lamina, diagramador de álbuns de fotografia. Comece por \
    resumo_do_album, ou abra um álbum com abrir_album ou novo_album. Lâminas, quadros, fotos e \
    opções contam a partir de 1, como na tela. Cada edição entra no histórico do Lamina como \
    \"IA: …\" e pode ser desfeita. O Lamina não salva sozinho: use salvar_album quando o fotógrafo \
    aprovar. As fotos são de clientes do fotógrafo: use as imagens só para o pedido dele.";

#[derive(Debug)]
struct State {
    agent: Agent,
    editor: Editor,
}

/// O servidor: o agente e o editor atrás de um `Mutex`, uma ferramenta de cada vez.
#[derive(Debug, Clone)]
pub struct Server {
    state: Arc<Mutex<State>>,
    images: bool,
}

impl Server {
    pub fn new(agent: Agent, editor: Editor) -> Self {
        let images = agent.images();
        Server {
            state: Arc::new(Mutex::new(State { agent, editor })),
            images,
        }
    }

    /// Com a configuração do fotógrafo; abre o álbum de `options`, se houver.
    pub(crate) fn for_photographer(options: &ServeOptions) -> Result<Self, McpError> {
        let (mut agent, mut editor) = photographer(options.allow.clone(), options.images);
        if let Some(album) = &options.album {
            agent.start_with(&mut editor, album)?;
        }
        Ok(Server::new(agent, editor))
    }

    pub(crate) fn images(&self) -> bool {
        self.images
    }

    /// Roda a chamada numa linha à parte; `progress` recebe (feitos, total).
    pub(crate) async fn run(
        &self,
        call: ToolCall,
        progress: impl Fn(usize, usize) + Send + Sync + 'static,
    ) -> Result<ToolReply, ToolError> {
        let state = Arc::clone(&self.state);
        let task = tokio::task::spawn_blocking(move || {
            let mut guard = state.lock().map_err(|_| stuck())?;
            let State { agent, editor } = &mut *guard;
            agent.run(editor, call, &progress)
        });
        task.await.map_err(|_| stuck())?
    }
}

/// Avisa o andamento, se o cliente pediu (com um `progressToken` na chamada).
async fn progress(
    context: &RequestContext<RoleServer>,
    token: Option<&ProgressToken>,
    done: usize,
    total: usize,
) {
    let Some(token) = token else {
        return;
    };
    let mut param = ProgressNotificationParam::new(token.clone(), done as f64);
    param.total = Some(total as f64);
    // Cliente que já fechou não precisa saber do andamento.
    let _ = context.peer.notify_progress(param).await;
}

fn stuck() -> ToolError {
    ToolError::Refused("A ferramenta parou no meio. Feche e abra o Lamina no cliente de IA.".into())
}

impl ServerHandler for Server {
    fn get_info(&self) -> ServerConfig {
        let capabilities = ServerCapabilities::builder()
            .enable_tools()
            .enable_prompts()
            .enable_resources()
            .build();
        let info = Implementation::new("lamina", env!("CARGO_PKG_VERSION")).with_title("Lamina");
        ServerConfig::new(capabilities)
            .with_server_info(info)
            .with_instructions(INSTRUCTIONS)
    }

    async fn list_tools(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListToolsResult, ErrorData> {
        let all = tools().into_iter().map(convert::tool).collect();
        Ok(ListToolsResult::with_all_items(all))
    }

    async fn call_tool(
        &self,
        request: CallToolRequestParams,
        context: RequestContext<RoleServer>,
    ) -> Result<CallToolResponse, ErrorData> {
        let args = request.arguments.map_or(Value::Null, Value::Object);
        let call = match ToolCall::parse(&request.name, args) {
            Ok(call) => call,
            Err(e) => return convert::failure(e).map(Into::into),
        };
        let token = context.meta.get_progress_token();
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<(usize, usize)>();
        let run = self.run(call, move |done, total| {
            // Sem cliente ouvindo, o andamento só se perde.
            let _ = tx.send((done, total));
        });
        tokio::pin!(run);
        let outcome = loop {
            tokio::select! {
                outcome = &mut run => break outcome,
                Some((done, total)) = rx.recv() => progress(&context, token.as_ref(), done, total).await,
            }
        };
        // O que ficou na fila quando a ferramenta terminou sai antes da resposta: nenhum aviso de
        // andamento se perde, e todos chegam antes do resultado.
        while let Ok((done, total)) = rx.try_recv() {
            progress(&context, token.as_ref(), done, total).await;
        }
        match outcome {
            Ok(reply) => Ok(convert::result(reply).into()),
            Err(e) => convert::failure(e).map(Into::into),
        }
    }

    async fn list_prompts(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListPromptsResult, ErrorData> {
        Ok(prompts::list())
    }

    async fn get_prompt(
        &self,
        request: GetPromptRequestParams,
        _context: RequestContext<RoleServer>,
    ) -> Result<GetPromptResponse, ErrorData> {
        prompts::get(request).map(Into::into)
    }

    async fn list_resources(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListResourcesResult, ErrorData> {
        Ok(resources::list())
    }

    async fn list_resource_templates(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListResourceTemplatesResult, ErrorData> {
        Ok(resources::templates(self.images))
    }

    async fn read_resource(
        &self,
        request: ReadResourceRequestParams,
        _context: RequestContext<RoleServer>,
    ) -> Result<ReadResourceResponse, ErrorData> {
        resources::read(self, &request.uri).await.map(Into::into)
    }
}
