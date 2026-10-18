//! Cenários da spec `presets` (PR) no editor: B1 a B9 e B11. O B10 roda o binário `lamina`.

use std::collections::HashMap;
use std::time::{Duration, Instant};

use lamina_core::testing::{TRIES, budget};
use lamina_core::{
    ColorSpace, ExportPrefs, FrameContent, OutputFormat, PageLimits, PhotoId, Project,
};

use super::common::Base;
use super::tl_common::{H, V, built, editor_at, spec};
use crate::presets::{Preset, PresetError, PresetsStore, builtin_presets};
use crate::respec::current_design;
use crate::{Command, Editor, Session, SpecFields, create_album, suggestion_within};

#[test]
fn pr_b1_presets_do_lamina() {
    let names: Vec<String> = PresetsStore::default()
        .all()
        .into_iter()
        .map(|p| p.name)
        .collect();
    assert_eq!(
        names,
        [
            "Quadrado 20 × 20",
            "Quadrado 25 × 25",
            "Quadrado 30 × 30",
            "Paisagem 30 × 20",
            "Paisagem 40 × 30",
            "Retrato 20 × 30"
        ]
    );
    // Os números de cada preset, escritos aqui (auditoria do Marco 3, M5).
    let sizes: Vec<(f64, f64)> = builtin_presets()
        .iter()
        .map(|p| (p.fields.width_cm, p.fields.height_cm))
        .collect();
    assert_eq!(
        sizes,
        [
            (20.0, 20.0),
            (25.0, 25.0),
            (30.0, 30.0),
            (30.0, 20.0),
            (40.0, 30.0),
            (20.0, 30.0)
        ]
    );
    for preset in builtin_presets() {
        let f = &preset.fields;
        assert_eq!(
            (f.dpi, f.bleed_mm, f.safe_mm),
            (300, 3.0, 10.0),
            "{}",
            preset.name
        );
        let spec = preset.fields.to_spec().unwrap();
        assert_eq!(spec.dpi().get(), 300, "{}", preset.name);
        assert_eq!((preset.limits.min, preset.limits.max), (Some(20), Some(80)));
        assert_eq!(preset.export, ExportPrefs::default());
    }
}

#[test]
fn pr_b2_novo_album_a_partir_de_um_preset() {
    let base = Base::with_photos(12, (300, 200));
    let preset = PresetsStore::default().find("Paisagem 30 × 20").unwrap();
    let new = crate::NewAlbum {
        spec: preset.fields.to_spec().unwrap(),
        limits: preset.limits,
        export: preset.export,
        ..base.new_album(Some(3))
    };
    let (built, _) = create_album(&new, &base.cache(), &|_| {}).unwrap();
    let project = built.project;
    assert_eq!(SpecFields::from_spec(&project.spec), preset.fields);
    let page = project.spec.page_rect(lamina_core::PageSide::Left);
    assert!((page.width.get() - 300.0).abs() < 1e-9 && (page.height.get() - 200.0).abs() < 1e-9);
    assert_eq!(project.spec.dpi().get(), 300);
    assert_eq!(
        (project.page_limits.min, project.page_limits.max),
        (Some(20), Some(80))
    );
    assert_eq!(project.page_limits, preset.limits);
    assert_eq!(project.export, ExportPrefs::default());
}

fn custom(name: &str) -> Preset {
    Preset {
        name: name.to_owned(),
        fields: SpecFields {
            width_cm: 25.0,
            height_cm: 25.0,
            ..SpecFields::default()
        },
        limits: PageLimits {
            min: Some(30),
            max: Some(100),
        },
        export: ExportPrefs {
            format: OutputFormat::Pdf,
            space: ColorSpace::AdobeRgb,
            pages: false,
        },
    }
}

#[test]
fn pr_b3_salvar_ler_de_novo_e_apagar() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("config").join("presets.json");
    let mut store = PresetsStore::load(&file);
    store.save(custom("Laboratório X")).unwrap();
    let again = PresetsStore::load(&file);
    assert_eq!(again.find("Laboratório X"), Some(custom("Laboratório X")));
    assert_eq!(again.warning, None);
    assert_eq!(
        store.save(custom("Quadrado 30 × 30")),
        Err(PresetError::Builtin)
    );
    assert_eq!(
        PresetError::Builtin.to_string(),
        "Esse nome é de um preset do Lamina; escolha outro."
    );
    assert_eq!(store.save(custom("  ")), Err(PresetError::EmptyName));
    let mut backwards = custom("Ao contrário");
    backwards.limits = PageLimits {
        min: Some(50),
        max: Some(20),
    };
    assert_eq!(store.save(backwards), Err(PresetError::LimitsOrder));
    assert!(store.rename("Laboratório X", "Laboratório Y").unwrap());
    assert!(store.remove("Laboratório Y").unwrap());
    assert!(PresetsStore::load(&file).user().is_empty());
}

#[test]
fn pr_b4_arquivo_de_presets_ilegivel() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("presets.json");
    std::fs::write(&file, "isto não é json").unwrap();
    let store = PresetsStore::load(&file);
    assert_eq!(store.all().len(), 6, "só os do Lamina");
    assert!(
        store
            .warning
            .as_deref()
            .is_some_and(|w| w.starts_with("Não foi possível ler os presets: ")),
        "{:?}",
        store.warning
    );
}

#[test]
fn pr_b5_abaixo_do_minimo_e_acima_do_maximo() {
    let limits = PageLimits {
        min: Some(20),
        max: Some(80),
    };
    let mut project = built(spec(30.0, 30.0), &[H; 18], 9);
    project.page_limits = limits;
    let session = Session::new_unsaved(project);
    assert_eq!(
        session.page_warning().as_deref(),
        Some("O álbum tem 18 páginas; o mínimo é 20.")
    );
    let mut project = built(spec(30.0, 30.0), &[H; 84], 42);
    project.page_limits = limits;
    assert_eq!(
        Session::new_unsaved(project).page_warning().as_deref(),
        Some("O álbum tem 84 páginas; o máximo é 80.")
    );
    let s = spec(30.0, 30.0);
    assert_eq!(crate::suggestion_for(30, &s), 8);
    assert_eq!(suggestion_within(30, &s, limits), 10);
}

#[test]
fn pr_b6_o_projeto_lembra_a_exportacao() {
    let dir = tempfile::tempdir().unwrap();
    let mut project = built(spec(30.0, 30.0), &[H, H, V, H], 1);
    project.export = custom("x").export;
    let mut session = Session::new_unsaved(project);
    let file = dir.path().join("album.lamina");
    session.save_as(&file).unwrap();
    let opened = Session::open(&file).unwrap();
    assert_eq!(opened.export_prefs(), custom("x").export);
    // Mudar na janela Exportar deixa o projeto com alterações, sem entrar no histórico.
    let mut opened = opened;
    opened.set_export_prefs(ExportPrefs::default());
    assert!(opened.is_dirty());
    assert!(!opened.can_undo());
    // Projeto antigo, sem os campos: JPEG em sRGB, sem limite de páginas.
    let mut old = built(spec(30.0, 30.0), &[H, H], 1);
    old.export = ExportPrefs::default();
    let json = old.to_json().unwrap();
    assert!(!json.contains("\"export\"") && !json.contains("page_limits"));
    let back = Project::from_json(&json).unwrap();
    assert_eq!(back.export, ExportPrefs::default());
    assert!(back.page_limits.is_empty());
}

/// (desenho de cada lâmina, centro do recorte de cada foto, fotos de cada lâmina).
type Snapshot = (
    Vec<Option<String>>,
    HashMap<PhotoId, (f64, f64)>,
    Vec<Vec<PhotoId>>,
);

fn snapshot(project: &Project) -> Snapshot {
    let designs = (0..project.spreads.len())
        .map(|i| current_design(project, i))
        .collect();
    let mut centers = HashMap::new();
    let mut photos = Vec::new();
    for spread in &project.spreads {
        let mut here = Vec::new();
        for frame in &spread.frames {
            if let FrameContent::Photo { photo, crop } = &frame.content {
                centers.insert(*photo, (crop.center_x, crop.center_y));
                here.push(*photo);
            }
        }
        here.sort();
        photos.push(here);
    }
    (designs, centers, photos)
}

fn album(n: usize) -> Editor {
    let photos: Vec<(u32, u32)> = (0..n * 4).map(|i| if i % 4 == 3 { V } else { H }).collect();
    editor_at(built(spec(30.0, 30.0), &photos, n), 0)
}

#[test]
fn pr_b7_trocar_para_o_mesmo_formato_outro_tamanho() {
    let mut editor = album(5);
    let before = editor.session().unwrap().project().clone();
    let (designs, centers, photos) = snapshot(&before);
    let new = spec(25.0, 25.0);
    let notice = editor.respec(new.clone(), PageLimits::default()).unwrap();
    assert_eq!(notice, None, "nenhuma lâmina mudou de desenho");
    let project = editor.session().unwrap().project().clone();
    assert_eq!(project.spec, new);
    let (designs_after, centers_after, photos_after) = snapshot(&project);
    assert_eq!(designs_after, designs);
    assert_eq!(photos_after, photos);
    for (photo, center) in centers {
        let after = centers_after[&photo];
        assert!((after.0 - center.0).abs() < 1e-9 && (after.1 - center.1).abs() < 1e-9);
    }
    for frame in project.spreads.iter().flat_map(|s| &s.frames) {
        let r = frame.rect;
        assert!(r.x() >= 0.0 && r.y() >= 0.0 && r.x() + r.width() <= 1.0 + 1e-9);
        assert!(r.y() + r.height() <= 1.0 + 1e-9);
    }
    assert!(editor.run(Command::Undo));
    assert_eq!(editor.session().unwrap().project(), &before);
}

#[test]
fn pr_b8_trocar_para_outra_proporcao() {
    let mut editor = album(4);
    let total = editor.session().unwrap().project().photos.len();
    let (_, _, photos) = snapshot(editor.session().unwrap().project());
    editor
        .respec(spec(30.0, 20.0), PageLimits::default())
        .unwrap();
    let session = editor.session().unwrap();
    let project = session.project();
    assert_eq!(project.spec, spec(30.0, 20.0));
    let (designs, _, photos_after) = snapshot(project);
    assert_eq!(photos_after, photos, "as mesmas fotos em cada lâmina");
    assert_eq!(photos_after.iter().map(Vec::len).sum::<usize>(), total);
    assert!(
        designs.iter().all(Option::is_some),
        "toda lâmina com um desenho das listas"
    );
    assert_eq!(
        crate::respec::changed_message(&[4, 7]).as_deref(),
        Some("2 lâminas mudaram de desenho: 4 e 7.")
    );
}

/// O desenho que não existe na proporção nova cai na 1ª opção, citada no aviso (auditoria do
/// Marco 3, M5): procura, entre as opções da lâmina 1, uma que some no 30 × 20.
#[test]
fn pr_b8_desenho_que_some_vira_a_primeira_opcao_com_aviso() {
    let mut editor = album(1);
    let count = editor.panel(crate::Part::Spread).options.len();
    let mut found = false;
    for i in 0..count {
        editor.apply_option(crate::Part::Spread, i);
        let notice = editor
            .respec(spec(30.0, 20.0), PageLimits::default())
            .unwrap();
        if let Some(text) = notice {
            assert_eq!(text, "1 lâmina mudou de desenho: 1.");
            found = true;
            break;
        }
        assert!(editor.run(Command::Undo));
    }
    assert!(found, "nenhuma das {count} opções sumiu no 30 × 20");
}

#[test]
fn pr_b9_pontas_que_mudam() {
    let mut editor = album(3);
    let total = editor.session().unwrap().project().photos.len();
    let single = spec(30.0, 30.0).with_edges(
        lamina_core::AlbumEdge::SinglePage,
        lamina_core::AlbumEdge::FullSpread,
    );
    editor.respec(single, PageLimits::default()).unwrap();
    let project = editor.session().unwrap().project();
    let (_, _, photos) = snapshot(project);
    assert_eq!(
        photos.iter().map(Vec::len).sum::<usize>(),
        total,
        "nenhuma foto some"
    );
    // Na 1ª lâmina, tudo na página direita.
    assert!(
        project.spreads[0]
            .frames
            .iter()
            .all(|f| f.rect.x() >= 0.5 - 1e-6)
    );
}

#[test]
fn pr_b11_trocar_rapido() {
    let mut best = Duration::MAX;
    for _ in 0..TRIES {
        let mut editor = album(40);
        let started = Instant::now();
        editor
            .respec(spec(25.0, 25.0), PageLimits::default())
            .unwrap();
        best = best.min(started.elapsed());
        if best < budget(2000) {
            break;
        }
    }
    assert!(best < budget(2000), "trocar a especificação levou {best:?}");
}
