//! As durações dos movimentos (spec MV, seção 4), em milissegundos. "Reduzir movimento" (regra
//! 14) zera todas: a tela recebe zero e muda de uma vez.

/// Quanto dura cada movimento, em ms.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Durations {
    /// Foto que continua na lâmina desliza do lugar antigo para o novo.
    pub slide: u32,
    /// Foto que entra ou sai esmaece no lugar.
    pub fade: u32,
    /// Ir para outra lâmina.
    pub navigate: u32,
    /// Faixa e painel acompanhando a lâmina atual.
    pub scroll: u32,
    /// O fantasma de uma foto solta pousa no quadro.
    pub land: u32,
    /// A miniatura dá lugar à foto nítida.
    pub sharp: u32,
    /// As lâminas vizinhas abrem espaço ao reordenar.
    pub reorder: u32,
    /// A faixa de layouts desce e sobe, e a lâmina muda de tamanho junto (spec FL, regra 12).
    pub band: u32,
}

impl Durations {
    /// As durações da spec, ou zero com "Reduzir movimento".
    pub fn new(reduce: bool) -> Self {
        let k = u32::from(!reduce);
        Durations {
            slide: 180 * k,
            fade: 120 * k,
            navigate: 200 * k,
            scroll: 250 * k,
            land: 150 * k,
            sharp: 120 * k,
            reorder: 150 * k,
            band: 200 * k,
        }
    }
}
