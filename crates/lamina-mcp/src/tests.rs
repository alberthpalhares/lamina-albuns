//! Cenários da spec IA que passam pelo protocolo, com um cliente MCP ligado ao servidor no mesmo
//! processo: IA-B17 (ferramentas, imagem e erro), IA-B19 (recursos e prompts) e a parte de IA-B3
//! que é dos recursos. Os demais moram no `lamina-editor`; o IA-B18, nos testes do `lamina`.

use std::path::{Path, PathBuf};

use base64::Engine as _;
use image::{Rgb, RgbImage};
use lamina_editor::agent::{Agent, AgentOptions, tools};
use lamina_editor::{Cache, Command, Editor, builtin_presets};
use rmcp::model::{
    CallToolRequestParams, CallToolResult, GetPromptRequestParams, ReadResourceRequestParams,
    ResourceContents,
};
use rmcp::service::RunningService;
use rmcp::{RoleClient, ServiceExt};
use serde_json::{Value, json};

use crate::Server;

type Client = RunningService<RoleClient, ()>;

/// Oito fotos de 600 × 400 em `<dir>/fotos`.
fn photos(dir: &Path) -> PathBuf {
    let folder = dir.join("fotos");
    std::fs::create_dir_all(&folder).unwrap();
    for i in 0..8u8 {
        RgbImage::from_fn(600, 400, |x, _| {
            Rgb([i * 30, u8::try_from(x % 256).unwrap(), 90])
        })
        .save(folder.join(format!("f{i}.jpg")))
        .unwrap();
    }
    folder
}

/// Um cliente ligado a um servidor novo, com `dir` permitida.
async fn connect(dir: &Path, images: bool) -> Client {
    let options = AgentOptions {
        roots: vec![dir.to_path_buf()],
        images,
        cache: Cache::new(dir.join("cache")),
        presets: builtin_presets(),
    };
    let server = Server::new(Agent::new(options), Editor::new());
    let (client_io, server_io) = tokio::io::duplex(1 << 22);
    tokio::spawn(async move {
        if let Ok(running) = rmcp::serve_server(server, server_io).await {
            let _ = running.waiting().await;
        }
    });
    ().serve(client_io).await.unwrap()
}

async fn call(client: &Client, name: &'static str, args: Value) -> CallToolResult {
    let Value::Object(args) = args else {
        panic!("argumentos devem ser um objeto");
    };
    let params = CallToolRequestParams::new(name).with_arguments(args);
    client.call_tool(params).await.unwrap()
}

/// O álbum de 2 lâminas criado pela própria IA.
async fn with_album(client: &Client, dir: &Path) {
    let folder = photos(dir);
    let args = json!({ "pasta_fotos": folder, "salvar_em": dir.join("album.lamina"), "laminas": 2, "montar": true });
    let created = call(client, "novo_album", args).await;
    assert_eq!(created.is_error, Some(false), "{created:?}");
}

fn text(result: &CallToolResult, i: usize) -> &str {
    &result.content[i].as_text().unwrap().text
}

#[tokio::test]
async fn ia_b17_o_protocolo() {
    let dir = tempfile::tempdir().unwrap();
    let client = connect(dir.path(), true).await;
    let listed = client.list_all_tools().await.unwrap();
    let names: Vec<&str> = listed.iter().map(|t| t.name.as_ref()).collect();
    let expected: Vec<&str> = tools().iter().map(|t| t.name).collect();
    assert_eq!(names, expected);
    assert!(
        listed
            .iter()
            .all(|t| t.input_schema.get("type") == Some(&json!("object")))
    );

    with_album(&client, dir.path()).await;
    let spread = call(&client, "ver_lamina", json!({ "lamina": 1 })).await;
    assert_eq!(spread.is_error, Some(false));
    assert!(text(&spread, 0).starts_with("Lâmina 1 com "));
    let data: Value = serde_json::from_str(text(&spread, 1)).unwrap();
    assert_eq!(spread.structured_content.as_ref(), Some(&data));
    let image = spread.content[2].as_image().unwrap();
    assert_eq!(image.mime_type, "image/png");
    let png = base64::engine::general_purpose::STANDARD
        .decode(&image.data)
        .unwrap();
    assert_eq!(&png[1..4], b"PNG");

    let refused = call(&client, "ver_lamina", json!({ "lamina": 99 })).await;
    assert_eq!(refused.is_error, Some(true));
    assert_eq!(
        text(&refused, 0),
        "O álbum tem 2 lâminas; não existe a lâmina 99."
    );
    let summary = call(&client, "resumo_do_album", json!({})).await;
    assert_eq!(
        summary.is_error,
        Some(false),
        "o servidor continua respondendo"
    );
    let params = CallToolRequestParams::new("ver_lamina")
        .with_arguments(json!({ "lamna": 1 }).as_object().unwrap().clone());
    let Err(bad) = client.call_tool(params).await else {
        panic!("parâmetro errado devia ser erro de protocolo");
    };
    assert!(bad.to_string().contains("lamna"), "{bad}");
}

#[tokio::test]
async fn ia_b19_recursos_e_prompts_prontos() {
    let dir = tempfile::tempdir().unwrap();
    let client = connect(dir.path(), true).await;
    with_album(&client, dir.path()).await;
    let resources = client.list_all_resources().await.unwrap();
    let uris: Vec<&str> = resources.iter().map(|r| r.uri.as_str()).collect();
    assert_eq!(
        uris,
        ["lamina://album", "lamina://esquema", "lamina://comandos"]
    );
    let templates = client.list_all_resource_templates().await.unwrap();
    let templates: Vec<&str> = templates.iter().map(|t| t.uri_template.as_str()).collect();
    assert_eq!(
        templates,
        ["lamina://lamina/{n}", "lamina://lamina/{n}/previa.png"]
    );
    let prompts = client.list_all_prompts().await.unwrap();
    let prompts: Vec<&str> = prompts.iter().map(|p| p.name.as_str()).collect();
    assert_eq!(
        prompts,
        [
            "revisar_para_impressao",
            "montar_album_da_pasta",
            "contar_a_historia",
            "legendar"
        ]
    );

    let read = client
        .read_resource(ReadResourceRequestParams::new("lamina://comandos"))
        .await
        .unwrap();
    let ResourceContents::TextResourceContents { text, .. } = &read.contents[0] else {
        panic!("os comandos vêm em texto");
    };
    let data: Value = serde_json::from_str(text).unwrap();
    let names: Vec<&str> = data["comandos"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c["nome"].as_str().unwrap())
        .collect();
    let registry: Vec<&str> = Command::ALL.iter().map(|c| c.name()).collect();
    assert_eq!(names, registry);
    let prompt = client
        .get_prompt(GetPromptRequestParams::new("revisar_para_impressao"))
        .await
        .unwrap();
    let message = prompt.messages[0].content.as_text().unwrap();
    assert!(message.text.contains("pre_impressao"), "{}", message.text);
}

#[tokio::test]
async fn ia_b3_sem_imagens_a_previa_some_dos_recursos() {
    let dir = tempfile::tempdir().unwrap();
    let client = connect(dir.path(), false).await;
    with_album(&client, dir.path()).await;
    let templates = client.list_all_resource_templates().await.unwrap();
    assert_eq!(templates.len(), 1);
    assert_eq!(templates[0].uri_template, "lamina://lamina/{n}");
    let preview = ReadResourceRequestParams::new("lamina://lamina/1/previa.png");
    let Err(refused) = client.read_resource(preview).await else {
        panic!("sem imagens, a prévia é recusada");
    };
    assert!(
        refused.to_string().contains("imagens estão desligadas"),
        "{refused}"
    );
    let spread = call(&client, "ver_lamina", json!({ "lamina": 1 })).await;
    assert!(spread.content.iter().all(|c| c.as_image().is_none()));
}
