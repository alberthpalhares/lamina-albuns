//! O giro da foto no quadro (spec AF, regras 2 a 4): o fator de cobertura, os cantos sempre
//! dentro da foto, arrastar com giro e o DPI efetivo (AF-B1, AF-B3, AF-B4 e AF-B5).

use super::turn::{cover_factor, screen_to_photo, turned_box};
use crate::{AlbumSpec, Crop, Dpi, FrameContent, Mm, PhotoAdjust, Project, RelRect};

/// Um vetor da foto levado de volta para a tela (o giro da foto, no sentido horário).
fn photo_to_screen((x, y): (f64, f64), degrees: f64) -> (f64, f64) {
    let (s, c) = degrees.to_radians().sin_cos();
    (x * c - y * s, x * s + y * c)
}

/// Os quatro cantos do quadro `frame` (largura, altura) girado, na foto, para o recorte `crop`.
fn corners(crop: &Crop, photo: (f64, f64), frame: (f64, f64), degrees: f64) -> Vec<(f64, f64)> {
    let r = crop.turned_region(photo.0, photo.1, frame, degrees);
    let (tw, _) = turned_box(frame.0, frame.1, degrees);
    let per = r.width / tw;
    let (cx, cy) = (r.x + r.width / 2.0, r.y + r.height / 2.0);
    [(-1.0, -1.0), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)]
        .iter()
        .map(|(sx, sy)| {
            let (qx, qy) = screen_to_photo(
                (sx * frame.0 / 2.0 * per, sy * frame.1 / 2.0 * per),
                degrees,
            );
            (cx + qx, cy + qy)
        })
        .collect()
}

#[test]
fn af_b1_cobertura_de_0_3_grau_num_quadro_3_2() {
    let k = cover_factor((6000.0, 4000.0), (3.0, 2.0), 0.3);
    assert!((k - 1.0079).abs() < 0.0001, "K = {k}");
    assert_eq!(cover_factor((6000.0, 4000.0), (3.0, 2.0), 0.0), 1.0);
}

#[test]
fn af_b3_cobertura_em_qualquer_angulo() {
    let photos = [(6000.0, 4000.0), (4000.0, 6000.0), (3000.0, 3000.0)];
    let frames = [(1.0, 1.0), (3.0, 2.0), (1.0, 3.0)];
    let centers = [(0.5, 0.5), (0.0, 0.0), (1.0, 1.0), (0.0, 1.0), (1.0, 0.3)];
    for photo in photos {
        for frame in frames {
            for degrees in [-45.0, -7.3, 0.1, 12.0, 45.0] {
                for (center_x, center_y) in centers {
                    for zoom in [1.0, 2.5] {
                        let crop = Crop {
                            zoom,
                            center_x,
                            center_y,
                        };
                        for (x, y) in corners(&crop, photo, frame, degrees) {
                            let inside = x >= -1e-6
                                && y >= -1e-6
                                && x <= photo.0 + 1e-6
                                && y <= photo.1 + 1e-6;
                            assert!(
                                inside,
                                "canto vazio: {photo:?} {frame:?} {degrees}° {crop:?} → ({x}, {y})"
                            );
                        }
                    }
                }
            }
        }
    }
}

#[test]
fn af_b4_arrastar_com_giro_segue_o_mouse() {
    let (photo, frame) = ((6000.0, 4000.0), (1.0, 1.0));
    let start = Crop {
        zoom: 2.0,
        ..Crop::default()
    };
    let moved = start.panned_turned(photo, frame, 10.0, (0.1, 0.0));
    // O ponto da foto que estava no centro do quadro foi para a direita na tela.
    let d = (
        (start.center_x - moved.center_x) * photo.0,
        (start.center_y - moved.center_y) * photo.1,
    );
    let (sx, sy) = photo_to_screen(d, 10.0);
    assert!(
        sx > 0.0 && sy.abs() < 1e-6 * photo.0,
        "andou ({sx}, {sy}) na tela"
    );
    // Arrastar muito não deixa vazio: a região girada continua dentro da foto.
    let far = start.panned_turned(photo, frame, 10.0, (50.0, -50.0));
    for (x, y) in corners(&far, photo, frame, 10.0) {
        assert!(x >= -1e-6 && y >= -1e-6 && x <= photo.0 + 1e-6 && y <= photo.1 + 1e-6);
    }
}

#[test]
fn af_b5_dpi_efetivo_cai_pelo_fator_de_cobertura() {
    let spec = AlbumSpec::new(Mm(300.0), Mm(300.0), Dpi::new(300).unwrap()).unwrap();
    let mut project = Project::new("teste", spec);
    let photo = project.add_photo("a.jpg", 3000, 2000);
    let spread = project.add_spread();
    let content = FrameContent::Photo {
        photo,
        crop: Crop::default(),
    };
    let rect = RelRect::new(0.1, 0.1, 0.3, 0.4).unwrap();
    project.add_frame(spread, rect, content).unwrap();
    let frame = project.spreads[0].frames[0].clone();
    let straight = project.effective_dpi(&frame).unwrap().unwrap();
    project.photos[0].adjust = PhotoAdjust {
        angle_tenths: 100,
        ..PhotoAdjust::default()
    };
    let turned = project.effective_dpi(&frame).unwrap().unwrap();
    let r = project.frame_rect(&frame);
    let k = cover_factor((3000.0, 2000.0), (r.width.get(), r.height.get()), 10.0);
    assert!(k > 1.1, "10° aproxima bem: K = {k}");
    assert!((straight / turned - k).abs() < 1e-9);
}
