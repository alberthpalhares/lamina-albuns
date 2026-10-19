//! IA-B18 (spec `controle-por-ia`): o `lamina mcp` rodando como processo só escreve o protocolo
//! no stdout, inclusive durante uma exportação com andamento.

#![allow(clippy::unwrap_used)]

mod common;

use std::io::Write as _;
use std::process::{Command, Stdio};

use common::{Base, text};
use serde_json::{Value, json};

#[test]
fn ia_b18_saida_limpa() {
    let base = Base::with_photos(6, (300, 200));
    let out = base.build("saida", &["--laminas", "2", "--nome", "album"]);
    assert!(out.status.success(), "{}", text(&out.stderr));
    let album = base.path("saida/album.lamina");
    let allowed = base.path("");

    let mut child = Command::new(env!("CARGO_BIN_EXE_lamina"))
        .args([
            "mcp",
            album.to_str().unwrap(),
            "--permitir",
            allowed.to_str().unwrap(),
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let export = json!({ "pasta": base.path("ia"), "formato": "jpeg" });
    let messages = [
        json!({ "jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {
            "protocolVersion": "2025-06-18", "capabilities": {},
            "clientInfo": { "name": "teste", "version": "1" } } }),
        json!({ "jsonrpc": "2.0", "method": "notifications/initialized" }),
        json!({ "jsonrpc": "2.0", "id": 2, "method": "tools/call", "params": {
            "name": "exportar", "arguments": export, "_meta": { "progressToken": 7 } } }),
        json!({ "jsonrpc": "2.0", "id": 3, "method": "tools/call", "params": {
            "name": "resumo_do_album", "arguments": {} } }),
    ];
    let mut stdin = child.stdin.take().unwrap();
    for message in &messages {
        writeln!(stdin, "{message}").unwrap();
    }
    // As respostas chegam antes de o cliente fechar: espera a última e fecha.
    let output = wait_for_answers(child, stdin);
    let lines: Vec<&str> = output.lines().filter(|l| !l.is_empty()).collect();
    let mut answered = Vec::new();
    for line in &lines {
        let message: Value = serde_json::from_str(line).unwrap_or_else(|e| panic!("{line}: {e}"));
        assert_eq!(message["jsonrpc"], "2.0", "{line}");
        if let Some(id) = message["id"].as_u64() {
            assert!(message["result"].is_object(), "{line}");
            answered.push(id);
        }
    }
    answered.sort_unstable();
    assert_eq!(answered, [1, 2, 3]);
    let progress = lines
        .iter()
        .filter(|l| l.contains("notifications/progress"))
        .count();
    assert_eq!(progress, 2, "um aviso de andamento por lâmina exportada");
    assert!(base.path("ia/lamina-01.jpg").exists());
    assert!(base.path("ia/lamina-02.jpg").exists());
}

/// Lê o stdout até as três respostas chegarem, fecha a entrada e devolve tudo o que saiu.
fn wait_for_answers(mut child: std::process::Child, stdin: std::process::ChildStdin) -> String {
    use std::io::BufRead as _;
    let stdout = child.stdout.take().unwrap();
    let mut reader = std::io::BufReader::new(stdout);
    let mut all = String::new();
    let mut answers = 0;
    while answers < 3 {
        let mut line = String::new();
        if reader.read_line(&mut line).unwrap() == 0 {
            break;
        }
        if serde_json::from_str::<Value>(&line).is_ok_and(|m| m["id"].is_u64()) {
            answers += 1;
        }
        all.push_str(&line);
    }
    drop(stdin);
    let mut rest = String::new();
    std::io::Read::read_to_string(&mut reader, &mut rest).unwrap();
    all.push_str(&rest);
    let status = child.wait().unwrap();
    assert!(status.success(), "o servidor terminou com {status}");
    all
}
