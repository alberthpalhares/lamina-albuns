//! As réguas do modo organizar (spec EG, seção 4): marcas na unidade da tela, contadas a partir do
//! corte (o mesmo zero dos campos X e Y), em frações da largura da lâmina com a sangria, como as
//! linhas do ímã. Com zoom, a tela só multiplica pela largura desenhada.

use lamina_core::Project;

use crate::new_project::Unit;

/// Uma marca da régua: onde fica (fração da largura da lâmina, a partir do canto com a sangria),
/// o número (vazio nas marcas pequenas) e se é grande.
#[derive(Debug, Clone, PartialEq)]
pub struct Tick {
    pub at: f64,
    pub label: String,
    pub major: bool,
}

/// As duas réguas: a de cima (x) e a do lado (y).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Rulers {
    pub xs: Vec<Tick>,
    pub ys: Vec<Tick>,
}

/// O passo das marcas pequenas e quantas pequenas há entre duas grandes, em mm.
fn steps(unit: Unit) -> (f64, u32) {
    match unit {
        Unit::Mm => (5.0, 10),
        Unit::Cm => (10.0, 5),
        Unit::Inch => (12.7, 4),
    }
}

/// As marcas de um eixo: de `from` a `to` mm a partir do corte, a lâmina começando `offset` mm
/// antes do corte, numa lâmina de `width` mm de largura.
fn axis(unit: Unit, (from, to): (f64, f64), offset: f64, width: f64) -> Vec<Tick> {
    let (step, every) = steps(unit);
    let mut ticks = Vec::new();
    // Da primeira marca dentro da sangria até a última; o laço para pelo tamanho, não por contador.
    let mut k = (from / step).ceil();
    while k * step <= to + 1e-9 {
        let mm = k * step;
        let major = k.rem_euclid(f64::from(every)).abs() < 1e-9;
        let label = if major && mm >= -1e-9 {
            unit.show(if mm.abs() < 1e-9 { 0.0 } else { mm })
        } else {
            String::new()
        };
        ticks.push(Tick {
            at: (mm + offset) / width,
            label,
            major,
        });
        k += 1.0;
    }
    ticks
}

/// As réguas da lâmina na unidade `unit`.
pub fn rulers(project: &Project, unit: Unit) -> Rulers {
    let (canvas, trim) = (project.spec.canvas_rect(), project.spec.trim_rect());
    let w = canvas.width.get();
    let (ox, oy) = (trim.x.get() - canvas.x.get(), trim.y.get() - canvas.y.get());
    Rulers {
        xs: axis(unit, (-ox, w - ox), ox, w),
        ys: axis(unit, (-oy, canvas.height.get() - oy), oy, w),
    }
}
