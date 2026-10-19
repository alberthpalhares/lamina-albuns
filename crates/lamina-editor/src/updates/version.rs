//! Números de versão `X.Y.Z` com o sufixo opcional de teste (`-beta.N`), na ordem do semver: a
//! versão de teste vem antes da estável do mesmo número (`0.2.0-beta.1` < `0.2.0`).

use std::cmp::Ordering;

/// O padrão é `0.0.0` (só antes de saber a versão que roda).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Version {
    core: [u64; 3],
    /// Partes do sufixo depois do `-`, separadas por ponto; vazio = estável.
    pre: Vec<String>,
}

impl Version {
    /// Lê `0.2.0`, `v0.2.0` ou `0.3.0-beta.1`. `None` se não for uma versão.
    pub fn parse(text: &str) -> Option<Version> {
        let text = text.trim();
        let text = text.strip_prefix('v').unwrap_or(text);
        let (core, pre) = match text.split_once('-') {
            Some((core, pre)) => (core, Some(pre)),
            None => (text, None),
        };
        let mut parts = core.split('.').map(|p| p.parse::<u64>().ok());
        let core = [parts.next()??, parts.next()??, parts.next()??];
        if parts.next().is_some() {
            return None;
        }
        let pre = match pre {
            Some(pre) if pre.is_empty() || pre.split('.').any(str::is_empty) => return None,
            Some(pre) => pre.split('.').map(str::to_owned).collect(),
            None => Vec::new(),
        };
        Some(Version { core, pre })
    }

    /// É uma versão de teste (tem sufixo)?
    pub fn is_prerelease(&self) -> bool {
        !self.pre.is_empty()
    }
}

impl std::fmt::Display for Version {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let [a, b, c] = self.core;
        write!(f, "{a}.{b}.{c}")?;
        if !self.pre.is_empty() {
            write!(f, "-{}", self.pre.join("."))?;
        }
        Ok(())
    }
}

impl Ord for Version {
    fn cmp(&self, other: &Self) -> Ordering {
        self.core
            .cmp(&other.core)
            .then_with(|| match (self.pre.is_empty(), other.pre.is_empty()) {
                (true, true) => Ordering::Equal,
                (true, false) => Ordering::Greater,
                (false, true) => Ordering::Less,
                (false, false) => compare_pre(&self.pre, &other.pre),
            })
    }
}

impl PartialOrd for Version {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// Partes numéricas comparam como número e vêm antes das de texto; com o começo igual, a mais
/// curta vem antes (regra do semver).
fn compare_pre(a: &[String], b: &[String]) -> Ordering {
    for (x, y) in a.iter().zip(b) {
        let order = match (x.parse::<u64>(), y.parse::<u64>()) {
            (Ok(x), Ok(y)) => x.cmp(&y),
            (Ok(_), Err(_)) => Ordering::Less,
            (Err(_), Ok(_)) => Ordering::Greater,
            (Err(_), Err(_)) => x.cmp(y),
        };
        if order != Ordering::Equal {
            return order;
        }
    }
    a.len().cmp(&b.len())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v(text: &str) -> Version {
        Version::parse(text).unwrap()
    }

    #[test]
    fn ordem_do_semver() {
        let order = [
            "0.1.0-beta.1",
            "0.1.0-beta.2",
            "0.1.0-beta.10",
            "0.1.0",
            "0.1.1",
            "0.2.0-beta.1",
            "0.2.0",
            "1.0.0",
        ];
        for pair in order.windows(2) {
            assert!(v(pair[0]) < v(pair[1]), "{} < {}", pair[0], pair[1]);
        }
        assert_eq!(v("v0.2.0"), v("0.2.0"));
        assert_eq!(v("0.3.0-beta.1").to_string(), "0.3.0-beta.1");
        assert!(v("0.3.0-beta.1").is_prerelease() && !v("0.3.0").is_prerelease());
    }

    #[test]
    fn texto_que_nao_e_versao() {
        for bad in [
            "",
            "1.0",
            "1.0.0.0",
            "a.b.c",
            "1.0.0-",
            "1.0.0-beta..1",
            "1.-1.0",
        ] {
            assert_eq!(Version::parse(bad), None, "{bad:?}");
        }
    }
}
