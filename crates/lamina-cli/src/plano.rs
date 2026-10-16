//! Comando `lamina plano`: importa uma pasta e mostra quais fotos vão em cada lâmina.

use std::path::{Path, PathBuf};

use anyhow::Result;
use lamina_core::{AlbumSpec, Dpi, Edges, Mm, Photo, PhotoId};
use lamina_layout::{PlanOptions, SpreadRole, plan_spreads};

use crate::importar::{import, project_from, relative};

pub fn run(folder: &Path, spreads: usize, edges: Edges, cache: Option<PathBuf>) -> Result<()> {
    let (report, _) = import(folder, cache)?;
    let spec = AlbumSpec::new(Mm::from_cm(30.0), Mm::from_cm(30.0), Dpi::new(300)?)?;
    let project = project_from(&report, "plano", spec);

    let plan = plan_spreads(&project.photos, spreads, edges, &PlanOptions::default())?;
    println!("{:<7} {:<8} {:<9} fotos", "lâmina", "momento", "papel");
    for (i, spread) in plan.iter().enumerate() {
        let role = role_name(spread.role);
        println!(
            "{:<7} {:<8} {:<9} {}",
            i + 1,
            spread.moment + 1,
            role,
            describe(folder, &project.photos, &spread.photos)
        );
    }
    let moments = plan.last().map_or(0, |s| s.moment + 1);
    println!(
        "\n{} lâminas, {moments} momento(s), {} fotos.",
        plan.len(),
        project.photos.len()
    );
    Ok(())
}

/// Papel da lâmina em português.
pub fn role_name(role: SpreadRole) -> &'static str {
    match role {
        SpreadRole::Highlight => "destaque",
        SpreadRole::Dense => "densa",
        SpreadRole::Single => "única",
    }
}

/// "a.jpg, b.jpg" ou, para muitas fotos, "a.jpg … f.jpg (6 fotos)".
fn describe(folder: &Path, photos: &[Photo], ids: &[PhotoId]) -> String {
    let name = |id: &PhotoId| {
        photos
            .iter()
            .find(|p| p.id == *id)
            .map_or_else(|| "?".to_owned(), |p| relative(folder, &p.path))
    };
    match ids {
        [] => String::new(),
        [a] => name(a),
        [a, b] => format!("{}, {}", name(a), name(b)),
        [first, .., last] => format!("{} … {} ({} fotos)", name(first), name(last), ids.len()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn papel_em_portugues() {
        assert_eq!(role_name(SpreadRole::Highlight), "destaque");
        assert_eq!(role_name(SpreadRole::Dense), "densa");
        assert_eq!(role_name(SpreadRole::Single), "única");
    }
}
