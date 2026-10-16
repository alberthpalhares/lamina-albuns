//! Textos do painel de informações.

use lamina_core::Project;
use lamina_editor::PhotoInfo;

/// Número com no máximo uma casa e vírgula: 30 → "30", 2,5 → "2,5".
fn number(v: f64) -> String {
    format!("{}", (v * 10.0).round() / 10.0).replace('.', ",")
}

/// "30 × 30 cm por página" e "300 DPI · sangria 3 mm".
pub fn album_info(project: &Project) -> String {
    let spec = &project.spec;
    format!(
        "{} × {} cm por página\n{} DPI · sangria {} mm",
        number(spec.page_width().get() / 10.0),
        number(spec.page_height().get() / 10.0),
        spec.dpi().get(),
        number(spec.bleed().outer.get())
    )
}

pub fn photo_text(info: &PhotoInfo) -> String {
    let uses = match info.uses {
        0 => "Não usada".to_owned(),
        1 => "Usada 1 vez".to_owned(),
        n => format!("Usada {n} vezes"),
    };
    format!(
        "{}\n{} × {} px\nCaptura: {}\nCâmera: {}\n{uses}",
        info.name, info.width, info.height, info.taken_at, info.camera
    )
}
