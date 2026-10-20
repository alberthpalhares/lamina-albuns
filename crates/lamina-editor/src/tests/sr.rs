//! Cenários da spec `sugestoes-e-registros` (SR): o formulário, o registro de erros e quando os
//! itens da Ajuda ficam disponíveis.

use std::fs;
use std::path::Path;

use super::tl_common::{editor_at, three_spreads};
use crate::error_log::{LOG_FILE, LOG_LIMIT, OLD_LOG_FILE, install_panic_hook};
use crate::feedback::{browser_failed, channel_name, system_name};
use crate::{Command, Editor, ErrorLog, FEEDBACK_FORM, HumanMessage, feedback_address};

fn read_log(folder: &Path) -> String {
    fs::read_to_string(folder.join(LOG_FILE)).unwrap()
}

fn damaged() -> HumanMessage {
    HumanMessage::new(
        "Este arquivo de álbum está danificado.",
        "Abra uma cópia de segurança do álbum.",
        "Não foi possível ler o projeto festa.lamina\nJSON do projeto inválido",
    )
}

#[test]
fn sr_b1_os_itens_da_ajuda_estao_sempre_disponiveis() {
    let help = [Command::SendFeedback, Command::OpenLogs];
    let empty = Editor::new();
    let mut open = editor_at(three_spreads(), 0);
    for command in help {
        assert!(empty.available(command), "{command:?} sem álbum");
        assert!(open.available(command), "{command:?} com álbum");
    }
    open.set_busy(true);
    for command in Command::ALL {
        let expected = command == Command::About || help.contains(&command);
        assert_eq!(open.available(command), expected, "{command:?} ocupado");
    }
}

#[test]
fn sr_b2_o_endereco_leva_so_versao_sistema_e_canal() {
    assert_eq!(
        feedback_address("0.1.0-beta.3", "windows-x86_64", "beta"),
        format!("{FEEDBACK_FORM}?versao=0.1.0-beta.3&sistema=windows-x86_64&canal=beta")
    );
    // Um valor com espaço ou acento vai codificado, sem quebrar o endereço.
    assert_eq!(
        feedback_address("1 é", "linux-x86_64", "estavel"),
        format!("{FEEDBACK_FORM}?versao=1%20%C3%A9&sistema=linux-x86_64&canal=estavel")
    );
    assert_eq!(channel_name(true), "beta");
    assert_eq!(channel_name(false), "estavel");
    let system = system_name();
    assert!(system.starts_with(std::env::consts::OS), "{system}");
    assert!(system.ends_with(std::env::consts::ARCH), "{system}");
}

#[test]
fn sr_b3_um_erro_mostrado_vai_para_o_registro() {
    let dir = tempfile::tempdir().unwrap();
    let folder = dir.path().join("registros");
    ErrorLog::with_home(folder.clone(), None).error(&damaged());
    let text = read_log(&folder);
    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(lines.len(), 3, "{text}");
    let first = lines[0];
    assert!(
        first.as_bytes()[..4].iter().all(u8::is_ascii_digit),
        "{first}"
    );
    assert!(
        first.ends_with(
            " UTC · erro · Este arquivo de álbum está danificado. · Abra uma cópia de segurança \
             do álbum."
        ),
        "{first}"
    );
    assert_eq!(
        lines[1],
        "    detalhe: Não foi possível ler o projeto festa.lamina"
    );
    assert_eq!(lines[2], "    detalhe: JSON do projeto inválido");
}

#[test]
fn sr_b4_o_registro_nao_passa_de_dois_arquivos() {
    let dir = tempfile::tempdir().unwrap();
    let folder = dir.path().to_path_buf();
    let big = "x".repeat(usize::try_from(LOG_LIMIT).unwrap() + 10);
    fs::write(folder.join(LOG_FILE), &big).unwrap();
    fs::write(folder.join(OLD_LOG_FILE), "velho").unwrap();
    ErrorLog::with_home(folder.clone(), None).error(&damaged());
    assert_eq!(fs::read_to_string(folder.join(OLD_LOG_FILE)).unwrap(), big);
    let new = read_log(&folder);
    assert!(new.contains("· erro · Este arquivo de álbum"), "{new}");
    assert!(!new.contains("xxx"), "{new}");
    assert_eq!(fs::read_dir(&folder).unwrap().count(), 2);
}

#[test]
fn sr_b5_registro_que_nao_grava_nao_atrapalha() {
    let dir = tempfile::tempdir().unwrap();
    let blocked = dir.path().join("registros");
    fs::write(&blocked, "um arquivo no lugar da pasta").unwrap();
    let log = ErrorLog::with_home(blocked.clone(), None);
    log.error(&damaged());
    log.start("0.1.0", "linux-x86_64", "beta");
    log.unexpected("índice fora do limite", "algum.rs:1");
    assert_eq!(
        fs::read_to_string(&blocked).unwrap(),
        "um arquivo no lugar da pasta"
    );
    assert!(log.ready_folder().is_err()); // qualquer erro: o do sistema muda entre eles
}

#[test]
fn sr_b6_falha_inesperada_vai_para_o_registro() {
    let dir = tempfile::tempdir().unwrap();
    let folder = dir.path().join("registros");
    install_panic_hook(ErrorLog::with_home(folder.clone(), None));
    let payload = std::panic::catch_unwind(|| panic!("índice fora do limite")).unwrap_err();
    // Volta ao gancho padrão: os outros testes não gravam neste registro.
    drop(std::panic::take_hook());
    assert_eq!(
        crate::human::panic_text(payload.as_ref()),
        "índice fora do limite"
    );
    let text = read_log(&folder);
    let line = text
        .lines()
        .find(|l| l.contains("índice fora do limite"))
        .unwrap();
    assert!(
        line.contains(" UTC · falha inesperada · índice fora do limite · "),
        "{line}"
    );
    assert!(line.contains("sr.rs:"), "{line}");
}

#[test]
fn sr_b7_abrir_a_pasta_de_registros_cria_a_pasta() {
    let dir = tempfile::tempdir().unwrap();
    let folder = dir.path().join("Lamina").join("registros");
    let log = ErrorLog::with_home(folder.clone(), None);
    assert_eq!(log.folder(), folder);
    assert_eq!(log.ready_folder().unwrap(), folder);
    assert!(folder.is_dir());
}

#[test]
fn sr_b8_o_registro_nao_leva_o_nome_de_login() {
    let dir = tempfile::tempdir().unwrap();
    let folder = dir.path().to_path_buf();
    let log = ErrorLog::with_home(folder.clone(), Some(Path::new(r"C:\Users\maria")));
    log.start("0.1.0", "windows-x86_64", "beta");
    log.error(&HumanMessage::new(
        "Não foi possível abrir o álbum \"festa.lamina\".",
        "Abra de novo.",
        r"Não foi possível ler o projeto C:\Users\maria\Fotos\festa.lamina: sem permissão",
    ));
    let text = read_log(&folder);
    assert!(text.contains(r"~\Fotos\festa.lamina"), "{text}");
    assert!(!text.contains("maria"), "{text}");
    let start = text.lines().next().unwrap();
    assert!(
        start.ends_with(" UTC · início · Lamina 0.1.0 · windows-x86_64 · canal beta"),
        "{start}"
    );
    // Com a pasta pessoal desta máquina, o nome de login também não aparece na abertura.
    let real = ErrorLog::new(dir.path().join("real"));
    real.start("0.1.0", &system_name(), "beta");
    let line = read_log(&dir.path().join("real"));
    let login = std::env::var("USERNAME")
        .or_else(|_| std::env::var("USER"))
        .unwrap_or_default();
    assert!(login.len() < 3 || !line.contains(&login), "{line}");
}

#[test]
fn sr_b9_navegador_que_nao_abre() {
    let address = feedback_address("0.1.0", "linux-x86_64", "beta");
    let message = browser_failed(&address);
    assert_eq!(message.title, "Não foi possível abrir o navegador.");
    // O texto do diálogo não se seleciona; o detalhe sim (auditoria da Fase 4, achado 13).
    assert_eq!(
        message.what_to_do,
        "Clique em Mostrar detalhes, copie o endereço e cole no navegador."
    );
    assert_eq!(message.details, address);
}
