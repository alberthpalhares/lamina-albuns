//! Rodízio de famílias nas primeiras posições e desenho geral de um candidato, para o rodízio
//! não mostrar variações quase iguais (spec VL, regra 6).

use std::collections::HashSet;

use super::rank::Entry;
use super::{Family, FamilyKind, PageLayout};

/// Posições em que vale o rodízio de famílias.
const ROTATION: usize = 12;
/// Nota mínima para entrar no rodízio, em fração da nota da primeira opção.
const ROTATION_FLOOR: f64 = 0.6;

impl Family {
    /// Mesmo desenho geral: mesma família, mesma área (página ou lâmina) e mesmo tipo de
    /// arranjo (linhas ou colunas), ainda que a divisão das fotos mude.
    pub fn same_outline(&self, other: &Family) -> bool {
        let kind = |p: &PageLayout| p.arrangement;
        let side = |p: &Option<PageLayout>| p.as_ref().map(kind);
        match (self, other) {
            (Family::Across, Family::Across) => true,
            (
                Family::PerPage {
                    left: l1,
                    right: r1,
                },
                Family::PerPage {
                    left: l2,
                    right: r2,
                },
            ) => side(l1) == side(l2) && side(r1) == side(r2),
            (Family::WholeSpread(a), Family::WholeSpread(b))
            | (Family::Mirrored(a), Family::Mirrored(b)) => kind(a) == kind(b),
            (
                Family::Hero {
                    scope: s1,
                    support: p1,
                    ..
                },
                Family::Hero {
                    scope: s2,
                    support: p2,
                    ..
                },
            ) => s1 == s2 && p1 == p2,
            (Family::BleedPage { side: s1, rest: a }, Family::BleedPage { side: s2, rest: b }) => {
                s1 == s2 && side(a) == side(b)
            }
            (
                Family::Mosaic {
                    scope: s1,
                    layout: a,
                },
                Family::Mosaic {
                    scope: s2,
                    layout: b,
                },
            ) => s1 == s2 && kind(a) == kind(b),
            // O mesmo modelo, espelhado ou não, é o mesmo desenho (spec MD, MD-B8).
            (Family::Model { name: a, .. }, Family::Model { name: b, .. }) => a == b,
            _ => false,
        }
    }
}

/// Nas 12 primeiras posições, uma família só repete depois que todas as famílias que ainda
/// restam já apareceram; um desenho geral já mostrado não volta (só com outra divisão ou outro
/// respiro); e só entram opções com nota de pelo menos 60% da primeira. Depois, a ordem da nota.
pub fn rotate<'a>(sorted: Vec<Entry>, family: impl Fn(&Entry) -> &'a Family) -> Vec<Entry> {
    let floor = sorted.first().map_or(0.0, |e| e.value * ROTATION_FLOOR);
    let mut rest: Vec<Option<Entry>> = sorted.into_iter().map(Some).collect();
    let mut out: Vec<Entry> = Vec::with_capacity(rest.len());
    let mut round: HashSet<FamilyKind> = HashSet::new();
    while out.len() < ROTATION {
        let fresh =
            |e: &Entry| e.value >= floor && !out.iter().any(|s| family(s).same_outline(family(e)));
        let remaining: HashSet<FamilyKind> = rest
            .iter()
            .flatten()
            .filter(|e| fresh(e))
            .map(|e| e.kind)
            .collect();
        if remaining.is_empty() {
            break;
        }
        if remaining.is_subset(&round) {
            round.clear();
        }
        let pick = rest.iter().position(|e| {
            e.as_ref()
                .is_some_and(|e| fresh(e) && !round.contains(&e.kind))
        });
        let Some(entry) = pick.and_then(|i| rest[i].take()) else {
            break;
        };
        round.insert(entry.kind);
        out.push(entry);
    }
    out.extend(rest.into_iter().flatten());
    out
}

#[cfg(test)]
mod tests {
    use super::super::{Arrangement, Scope, SupportSide};
    use super::*;

    fn rows(groups: &[usize]) -> PageLayout {
        PageLayout {
            arrangement: Arrangement::Rows,
            groups: groups.to_vec(),
        }
    }

    #[test]
    fn so_a_divisao_diferente_e_o_mesmo_desenho() {
        let hero = |groups: &[usize]| Family::Hero {
            scope: Scope::Spread,
            support: SupportSide::Right,
            layout: rows(groups),
        };
        assert!(hero(&[3, 4, 4]).same_outline(&hero(&[4, 3, 4])));
        let below = Family::Hero {
            scope: Scope::Spread,
            support: SupportSide::Below,
            layout: rows(&[3, 4, 4]),
        };
        assert!(!hero(&[3, 4, 4]).same_outline(&below));
        assert!(!hero(&[3]).same_outline(&Family::WholeSpread(rows(&[3]))));
    }
}
