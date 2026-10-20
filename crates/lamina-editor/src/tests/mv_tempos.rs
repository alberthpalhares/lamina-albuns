//! `LAMINA_TEMPOS=1` (spec MV, seção 4): a medição de cada comando e de cada rajada de quadros.

use std::time::Duration;

use crate::motion::Timings;

fn ms(n: u64) -> Duration {
    Duration::from_millis(n)
}

fn ligada() -> Timings {
    Timings::from_value(Some("1"))
}

#[test]
fn medicao_so_liga_com_o_valor_1() {
    assert!(ligada().is_on());
    assert!(Timings::from_value(Some(" 1 ")).is_on());
    assert!(!Timings::from_value(Some("0")).is_on());
    assert!(!Timings::from_value(Some("sim")).is_on());
    assert!(!Timings::from_value(None).is_on());
}

#[test]
fn medicao_desligada_nao_escreve_nada() {
    let mut t = Timings::from_value(None);
    t.begin("Próximo layout", ms(0));
    t.editor_done(ms(2));
    t.screen_done(ms(5));
    assert!(t.frame(ms(10), ms(12)).is_empty());
    assert!(t.tick(ms(5_000)).is_empty());
}

#[test]
fn primeiro_quadro_depois_do_comando_da_a_linha_do_comando() {
    let mut t = ligada();
    t.begin("Próximo layout", ms(100));
    t.editor_done(ms(103));
    t.screen_done(ms(110));
    let lines = t.frame(ms(115), ms(118));
    assert_eq!(
        lines,
        vec!["Próximo layout: editor 3,0 ms · tela 7,0 ms · 1º quadro 18,0 ms"]
    );
    // O segundo quadro já não é o primeiro: nada a dizer até a rajada acabar.
    assert!(t.frame(ms(130), ms(132)).is_empty());
}

#[test]
fn rajada_termina_depois_de_um_intervalo_sem_quadros() {
    let mut t = ligada();
    t.begin("Próxima lâmina", ms(0));
    t.editor_done(ms(1));
    t.screen_done(ms(2));
    t.frame(ms(5), ms(10));
    for k in 1..=10 {
        assert!(t.frame(ms(10 + k * 16), ms(12 + k * 16)).is_empty());
    }
    // Ainda dentro do intervalo: a rajada continua aberta.
    assert!(t.tick(ms(180)).is_empty());
    let lines = t.tick(ms(400));
    assert_eq!(lines.len(), 1);
    let line = &lines[0];
    assert!(line.contains("11 quadro(s) em 162,0 ms"), "{line}");
    assert!(line.contains("62 q/s"), "{line}");
    assert!(line.contains("maior intervalo 18,0 ms"), "{line}");
    assert!(line.contains("desenho até 5,0 ms"), "{line}");
    assert!(line.ends_with("depois de Próxima lâmina"), "{line}");
}

#[test]
fn quadros_sem_comando_aparecem_como_desenho_sem_comando() {
    let mut t = ligada();
    assert!(t.frame(ms(0), ms(2)).is_empty());
    t.frame(ms(16), ms(18));
    let lines = t.tick(ms(1_000));
    assert_eq!(lines.len(), 1);
    assert!(lines[0].contains("2 quadro(s)"), "{}", lines[0]);
    assert!(lines[0].contains("sem comando"), "{}", lines[0]);
}

#[test]
fn comando_novo_fecha_a_rajada_anterior() {
    let mut t = ligada();
    t.begin("Espelhar", ms(0));
    t.editor_done(ms(1));
    t.screen_done(ms(2));
    t.frame(ms(3), ms(5));
    t.frame(ms(20), ms(22));
    t.begin("Desfazer", ms(30));
    t.editor_done(ms(31));
    t.screen_done(ms(32));
    let lines = t.frame(ms(36), ms(38));
    assert_eq!(lines.len(), 2, "{lines:?}");
    assert!(lines[0].contains("depois de Espelhar"), "{}", lines[0]);
    assert!(lines[1].starts_with("Desfazer: "), "{}", lines[1]);
}

#[test]
fn comando_que_nao_desenha_e_relatado_depois_de_um_segundo() {
    let mut t = ligada();
    t.begin("Salvar", ms(0));
    t.editor_done(ms(4));
    assert!(t.tick(ms(500)).is_empty());
    let lines = t.tick(ms(1_200));
    assert_eq!(
        lines,
        vec!["Salvar: editor 4,0 ms · tela — ms · 1º quadro nenhum"]
    );
}
