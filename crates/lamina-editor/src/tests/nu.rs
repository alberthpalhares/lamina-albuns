//! Cenários da spec `campos-de-numero` (NU): as setas ↑ e ↓ andam o passo do campo.

use crate::number_step::{Step, step_number};

const MM: Step = Step {
    step: 1.0,
    min: 0.0,
    max: 10_000.0,
};

#[test]
fn nu_b1_a_seta_anda_o_passo() {
    assert_eq!(step_number("30", 1, MM).as_deref(), Some("31"));
    assert_eq!(step_number("31", -1, MM).as_deref(), Some("30"));
    assert_eq!(step_number("30", -1, MM).as_deref(), Some("29"));
    assert_eq!(step_number(" 30 ", 1, MM).as_deref(), Some("31"));
}

#[test]
fn nu_b2_shift_anda_dez() {
    assert_eq!(step_number("30", 10, MM).as_deref(), Some("40"));
    assert_eq!(step_number("30", -10, MM).as_deref(), Some("20"));
}

#[test]
fn nu_b3_virgula_e_casas() {
    let spacing = Step {
        step: 0.1,
        min: 0.5,
        max: 3.0,
    };
    assert_eq!(step_number("1,2", 1, spacing).as_deref(), Some("1,3"));
    assert_eq!(step_number("12,5", 1, MM).as_deref(), Some("13,5"));
    // Com ponto, devolve com ponto; o passo decimal ganha a casa que faltava.
    assert_eq!(step_number("12.5", -1, MM).as_deref(), Some("11.5"));
    assert_eq!(step_number("1", 1, spacing).as_deref(), Some("1,1"));
    // O passo vem da janela em f32: 0,1 chega como 0,10000000149 e continua dando uma casa.
    let from_window = Step {
        step: f64::from(0.1_f32),
        min: 0.0,
        max: 10.0,
    };
    assert_eq!(step_number("0,3", 1, from_window).as_deref(), Some("0,4"));
    let inches = Step {
        step: f64::from(0.05_f32),
        ..from_window
    };
    assert_eq!(step_number("0,1", 1, inches).as_deref(), Some("0,15"));
    // A unidade escrita junto fica, e o menos tipográfico também.
    assert_eq!(step_number("12,0°", 1, MM).as_deref(), Some("13,0°"));
    assert_eq!(step_number("150%", 10, MM).as_deref(), Some("160%"));
    let angle = Step {
        step: 1.0,
        min: -180.0,
        max: 180.0,
    };
    assert_eq!(
        step_number("\u{2212}1,5°", -1, angle).as_deref(),
        Some("\u{2212}2,5°")
    );
}

#[test]
fn nu_b4_dentro_dos_limites() {
    let scale = Step {
        step: 1.0,
        min: 100.0,
        max: 800.0,
    };
    assert_eq!(step_number("799", 10, scale).as_deref(), Some("800"));
    assert_eq!(step_number("105", -10, scale).as_deref(), Some("100"));
    assert_eq!(step_number("abc", 1, scale), None);
    assert_eq!(step_number("", 1, scale), None);
    let angle = Step {
        step: 1.0,
        min: -180.0,
        max: 180.0,
    };
    assert_eq!(step_number("1", -1, angle).as_deref(), Some("0"));
    assert_eq!(step_number("-0,5", 1, angle).as_deref(), Some("0,5"));
}
