//! Checagem de camadas (ADR 0001): um crate só pode depender de crates de camadas **abaixo**
//! dele. Assim o núcleo nunca passa a depender da interface por acidente.

use std::process::Command;

use serde_json::Value;

/// Camada de cada crate do workspace. Crate novo precisa entrar aqui.
pub const LAYERS: &[(&str, u8)] = &[
    ("lamina-core", 0),
    ("lamina-media", 1),
    ("lamina-layout", 1),
    ("lamina-render", 1),
    ("lamina-export", 2),
    ("lamina-editor", 3),
    ("lamina-cli", 4),
    ("lamina-app", 4),
];

/// Dependências externas que os binários (camada 4) podem ter: eles só traduzem entrada e
/// saída (regra 1). O resto vem pelas bibliotecas (auditoria do Marco 1, M4).
pub const BINARY_EXTERNALS: &[&str] = &["anyhow", "clap", "rfd", "slint", "slint-build"];

/// Camada dos binários.
const BINARY_LAYER: u8 = 4;

fn layer_of(name: &str) -> Option<u8> {
    LAYERS.iter().find(|(n, _)| *n == name).map(|(_, l)| *l)
}

/// Devolve as violações encontradas em pares (crate, dependências).
pub fn violations(crates: &[(String, Vec<String>)]) -> Vec<String> {
    let mut found = Vec::new();
    for (name, deps) in crates {
        if !name.starts_with("lamina-") {
            continue;
        }
        let Some(own) = layer_of(name) else {
            found.push(format!(
                "{name} não tem camada definida em xtask/src/layers.rs"
            ));
            continue;
        };
        for dep in deps.iter().filter(|d| d.starts_with("lamina-")) {
            match layer_of(dep) {
                Some(layer) if layer < own => {}
                Some(layer) => found.push(format!(
                    "{name} (camada {own}) depende de {dep} (camada {layer}): só pode depender de camadas abaixo"
                )),
                None => found.push(format!("{dep} não tem camada definida")),
            }
        }
    }
    found
}

/// Binários com dependência externa (normal, não de teste) fora de `BINARY_EXTERNALS`.
pub fn binary_violations(crates: &[(String, Vec<String>)]) -> Vec<String> {
    crates
        .iter()
        .filter(|(name, _)| layer_of(name) == Some(BINARY_LAYER))
        .flat_map(|(name, deps)| {
            deps.iter()
                .filter(|d| !d.starts_with("lamina-") && !BINARY_EXTERNALS.contains(&d.as_str()))
                .map(move |d| {
                    format!("{name} (binário) depende de {d}: binários só traduzem entrada e saída; leve o trabalho para uma biblioteca")
                })
        })
        .collect()
}

/// (nome, todas as dependências, só as normais) de cada pacote do workspace.
type Package = (String, Vec<String>, Vec<String>);

fn workspace_crates() -> Result<Vec<Package>, String> {
    let output = Command::new(env!("CARGO"))
        .args(["metadata", "--no-deps", "--format-version", "1"])
        .output()
        .map_err(|e| format!("não foi possível rodar cargo metadata: {e}"))?;
    let json: Value = serde_json::from_slice(&output.stdout)
        .map_err(|e| format!("saída inválida do cargo metadata: {e}"))?;
    let packages = json["packages"]
        .as_array()
        .ok_or("cargo metadata sem lista de pacotes")?;
    Ok(packages
        .iter()
        .map(|p| {
            let name = p["name"].as_str().unwrap_or_default().to_owned();
            let list = p["dependencies"].as_array().cloned().unwrap_or_default();
            let names = |normal_only: bool| -> Vec<String> {
                list.iter()
                    .filter(|d| !normal_only || d["kind"].is_null())
                    .filter_map(|d| d["name"].as_str().map(str::to_owned))
                    .collect()
            };
            (name, names(false), names(true))
        })
        .collect())
}

pub fn run() -> bool {
    let crates = match workspace_crates() {
        Ok(crates) => crates,
        Err(error) => {
            eprintln!("{error}");
            return false;
        }
    };
    let all: Vec<(String, Vec<String>)> = crates
        .iter()
        .map(|(n, deps, _)| (n.clone(), deps.clone()))
        .collect();
    let normal: Vec<(String, Vec<String>)> = crates
        .iter()
        .map(|(n, _, deps)| (n.clone(), deps.clone()))
        .collect();
    let mut found = violations(&all);
    found.extend(binary_violations(&normal));
    for violation in &found {
        eprintln!("  ✘ {violation}");
    }
    if found.is_empty() {
        println!("  camadas ok ({} crates)", crates.len());
    }
    found.is_empty()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn krate(name: &str, deps: &[&str]) -> (String, Vec<String>) {
        (
            name.to_owned(),
            deps.iter().map(|d| (*d).to_owned()).collect(),
        )
    }

    #[test]
    fn dependencia_para_baixo_e_permitida() {
        let crates = [
            krate("lamina-export", &["lamina-render", "lamina-core", "serde"]),
            krate("lamina-cli", &["lamina-export"]),
        ];
        assert!(violations(&crates).is_empty());
    }

    /// Proveta: o núcleo dependendo da interface tem que reprovar.
    #[test]
    fn nucleo_dependendo_da_interface_reprova() {
        let found = violations(&[krate("lamina-core", &["lamina-app"])]);
        assert_eq!(found.len(), 1);
        assert!(found[0].contains("lamina-core (camada 0) depende de lamina-app (camada 4)"));
    }

    #[test]
    fn mesma_camada_reprova() {
        let found = violations(&[krate("lamina-layout", &["lamina-media"])]);
        assert_eq!(found.len(), 1);
    }

    /// Proveta do M4: binário com dependência de biblioteca (ex.: `image`) reprova.
    #[test]
    fn binario_com_dependencia_de_biblioteca_reprova() {
        let found = binary_violations(&[
            krate("lamina-cli", &["anyhow", "clap", "lamina-export", "image"]),
            krate("lamina-export", &["image"]),
        ]);
        assert_eq!(found.len(), 1);
        assert!(found[0].starts_with("lamina-cli (binário) depende de image"));
    }

    #[test]
    fn crate_sem_camada_reprova() {
        let found = violations(&[krate("lamina-novo", &[])]);
        assert_eq!(
            found,
            ["lamina-novo não tem camada definida em xtask/src/layers.rs"]
        );
    }
}
