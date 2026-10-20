//! Arrastar (spec MV, regra 12): o fantasma, o destaque igual ao destino, o pouso e o reordenar
//! lâminas (MV-B20 a MV-B23).

use lamina_core::{FrameContent, PhotoId, Project};

use super::tl_common::{H, V, built, editor_at, spec};
use crate::motion::{GHOST_PX, drop_target, frame_ghost, photo_ghost, reorder};
use crate::view::{Area, FrameKind, spread_view};

fn album(spreads: usize) -> Project {
    let photos: Vec<(u32, u32)> = (0..spreads * 4)
        .map(|i| if i % 4 == 3 { V } else { H })
        .collect();
    built(spec(30.0, 30.0), &photos, spreads)
}

fn frames(project: &Project, index: usize) -> Vec<crate::view::FrameView> {
    let canvas = project.spec.canvas_rect();
    let area = Area {
        width: 1.0,
        height: canvas.height.get() / canvas.width.get(),
        margin: 0.0,
    };
    spread_view(project, index, area, &[]).unwrap().frames
}

#[test]
fn mv_b20_fantasma_com_o_recorte_e_o_selo() {
    let mut project = album(4);
    // A foto A, no quadro 0 da lâmina 2, recortada com zoom 2.
    let a = match &mut project.spreads[1].frames[0].content {
        FrameContent::Photo { photo, crop } => {
            crop.zoom = 2.0;
            *photo
        }
        _ => panic!("quadro de foto"),
    };
    let ghost = frame_ghost(&project, 1, 0).unwrap();
    let view = &frames(&project, 1)[0];
    let FrameKind::Photo { source, .. } = &view.kind else {
        panic!("foto")
    };
    assert_eq!(ghost.photo, a);
    assert_eq!(&ghost.source, source, "a parte que está no quadro");
    assert!(ghost.source.width < 0.6, "com zoom 2, uma parte da foto");
    let aspect = view.rect.width / view.rect.height;
    assert!(
        (ghost.width / ghost.height - aspect).abs() < 1e-9,
        "na proporção do quadro"
    );
    assert!((ghost.width.max(ghost.height) - GHOST_PX).abs() < 1e-9);
    assert_eq!(ghost.badge, None);
    // Cinco fotos selecionadas no navegador: arrastar uma delas leva o selo "5".
    let five: Vec<PhotoId> = project.photos.iter().take(5).map(|p| p.id).collect();
    let many = photo_ghost(&project, five[2], &five).unwrap();
    assert_eq!(many.badge, Some(5));
    // Uma foto fora da seleção vai sozinha.
    let alone = photo_ghost(&project, project.photos[9].id, &five).unwrap();
    assert_eq!(alone.badge, None);
}

#[test]
fn mv_b21_destaque_igual_ao_destino() {
    let mut editor = editor_at(album(4), 1);
    assert!(editor.new_text(None));
    let s = editor.session().unwrap();
    let (project, index) = (s.project(), s.current());
    let all = frames(project, index);
    let text = all
        .iter()
        .position(|f| matches!(f.kind, FrameKind::Text { .. }))
        .unwrap();
    let t = all[text].rect;
    let (cx, cy) = (t.x + t.width / 2.0, t.y + t.height / 2.0);
    // Sobre o texto (por cima de uma foto): o destaque é o quadro que a soltura usaria.
    let (hit, rect) = drop_target(project, index, cx, cy).unwrap();
    assert_eq!(Some(hit), crate::view::frame_at(project, index, cx, cy));
    assert_eq!(hit, text);
    assert_eq!(rect, t);
    // Fora da lâmina, nenhum.
    assert_eq!(drop_target(project, index, 1.5, cy), None);
    assert_eq!(drop_target(project, index, -0.2, -0.2), None);
}

#[test]
fn mv_b22_o_fantasma_pousa_no_quadro() {
    let project = album(4);
    let all = frames(&project, 1);
    let r = all[1].rect;
    let (frame, landing) =
        drop_target(&project, 1, r.x + r.width / 2.0, r.y + r.height / 2.0).unwrap();
    assert_eq!(frame, 1);
    assert_eq!(landing, r);
    // Fora de qualquer quadro (na margem de fora da lâmina): não há pouso.
    assert_eq!(drop_target(&project, 1, 2.0, 2.0), None);
}

#[test]
fn mv_b23_reordenar_laminas() {
    // A lâmina 2 até o vão entre a 5 e a 6, numa faixa de 8.
    let r = reorder(1, 5, 8);
    assert_eq!(r.to, 4, "a posição 5");
    assert_eq!(
        r.shift,
        vec![0, 0, -1, -1, -1, 0, 0, 0],
        "as lâminas 3 a 5 recuam"
    );
    // Para trás: a lâmina 6 até o vão entre a 1 e a 2.
    let back = reorder(5, 1, 8);
    assert_eq!(back.to, 1);
    assert_eq!(back.shift, vec![0, 1, 1, 1, 1, 0, 0, 0]);
    // Soltar no próprio lugar não move nada.
    let same = reorder(3, 3, 8);
    assert_eq!(same.to, 3);
    assert!(same.shift.iter().all(|s| *s == 0));
    assert_eq!(reorder(3, 4, 8).to, 3);
}
