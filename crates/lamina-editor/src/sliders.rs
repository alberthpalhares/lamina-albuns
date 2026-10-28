//! As contas dos controles deslizantes (spec CD): a escala e o zoom proporcionais ao logaritmo,
//! para o ajuste fino perto de 100% não ficar espremido no começo do trilho, e o giro de 90° que
//! dá a volta em ±180°.

/// O valor numa posição `pos` do trilho (0 a 1), entre `min` e `max` (ambos maiores que zero):
/// cada décimo do trilho multiplica o valor pelo mesmo fator.
pub fn log_value(pos: f64, min: f64, max: f64) -> f64 {
    let pos = pos.clamp(0.0, 1.0);
    min * (max / min).powf(pos)
}

/// A posição do trilho (0 a 1) de `value`, o caminho de volta de [`log_value`].
pub fn log_position(value: f64, min: f64, max: f64) -> f64 {
    if max <= min || value <= min {
        return 0.0;
    }
    ((value / min).ln() / (max / min).ln()).clamp(0.0, 1.0)
}

/// Meia volta, em décimos de grau.
pub const HALF_TURN_TENTHS: i32 = 1800;

/// O ângulo `tenths` girado mais `delta` décimos, dando a volta: passar de 180° continua do lado
/// de −180° (170° + 90° = −100°; spec CD, regra 2).
pub fn wrap_turn(tenths: i32, delta: i32) -> i32 {
    let full = 2 * HALF_TURN_TENTHS;
    let x = (tenths + delta).rem_euclid(full);
    if x > HALF_TURN_TENTHS { x - full } else { x }
}
