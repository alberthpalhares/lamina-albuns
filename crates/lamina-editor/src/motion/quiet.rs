//! Dois relógios sem janela (spec FB, regras 6 e 7): o silêncio depois de a janela mudar de
//! tamanho, que desliga a animação da lâmina e junta os redesenhos num só, e a espera antes de a
//! barra da lâmina sumir. Os tempos são em ms, contados de um ponto qualquer (a janela usa o
//! início do programa).

/// Quanto tempo sem mudança de tamanho até a lâmina voltar a animar (regra 6).
pub const RESIZE_QUIET_MS: u64 = 150;
/// Quanto tempo a barra da lâmina espera depois que o ponteiro sai (regra 7).
pub const BAR_GRACE_MS: u64 = 500;

/// O silêncio depois de uma sequência de mudanças: ocupado até `ms` depois da última, e um aviso
/// só quando o silêncio chega.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Quiet {
    ms: u64,
    last: Option<u64>,
}

impl Quiet {
    pub fn new(ms: u64) -> Self {
        Quiet { ms, last: None }
    }

    /// Mais uma mudança em `now`.
    pub fn poke(&mut self, now: u64) {
        self.last = Some(now);
    }

    /// Ainda dentro da sequência: menos de `ms` desde a última mudança.
    pub fn busy(&self, now: u64) -> bool {
        self.last.is_some_and(|last| now < last + self.ms)
    }

    /// Quando o silêncio chega, se há uma sequência aberta.
    pub fn due(&self) -> Option<u64> {
        self.last.map(|last| last + self.ms)
    }

    /// `true` uma vez só, na primeira pergunta depois do silêncio; fecha a sequência.
    pub fn settle(&mut self, now: u64) -> bool {
        match self.due() {
            Some(due) if now >= due => {
                self.last = None;
                true
            }
            _ => false,
        }
    }
}

/// O ponteiro sobre a lâmina, a barra ou o vão entre elas, com a espera depois de sair.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct HoverGrace {
    inside: bool,
    left_at: Option<u64>,
}

impl HoverGrace {
    /// O ponteiro entrou (`true`) ou saiu de tudo (`false`) em `now`.
    pub fn set(&mut self, inside: bool, now: u64) {
        if self.inside && !inside {
            self.left_at = Some(now);
        }
        self.inside = inside;
    }

    /// A barra aparece agora? Durante um arrasto, nunca (regra 15 do rolo).
    pub fn shown(&self, now: u64, dragging: bool) -> bool {
        if dragging {
            return false;
        }
        self.inside || self.left_at.is_some_and(|at| now < at + BAR_GRACE_MS)
    }

    /// Quando perguntar de novo, para a barra sumir no fim da espera.
    pub fn due(&self) -> Option<u64> {
        match (self.inside, self.left_at) {
            (false, Some(at)) => Some(at + BAR_GRACE_MS),
            _ => None,
        }
    }
}

/// Até quanto tempo depois de um clique no cabeçalho de uma faixa o segundo toque é o do clique
/// duplo, e é ignorado (spec FX, regra 9).
pub const DOUBLE_CLICK_MS: u64 = 400;

/// Os cliques nos cabeçalhos das faixas: o clique simples e o duplo recolhem e abrem, uma vez só.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct HeaderClicks {
    last: Option<(u8, u64)>,
}

impl HeaderClicks {
    /// Um clique no cabeçalho `strip` em `now`: `true` se ele recolhe ou abre a faixa; o segundo
    /// toque de um clique duplo na mesma faixa não conta.
    pub fn accept(&mut self, strip: u8, now: u64) -> bool {
        let double = self
            .last
            .is_some_and(|(s, at)| s == strip && now < at + DOUBLE_CLICK_MS);
        self.last = if double { None } else { Some((strip, now)) };
        !double
    }
}
