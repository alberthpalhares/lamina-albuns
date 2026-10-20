//! As réguas e a paleta do modo organizar (spec EG, seções 3 e 4): o zero no corte, as marcas na
//! unidade da tela (EG-B22) e desenhar um quadro ou um texto arrastando.

use lamina_core::FrameContent;

use super::eg_imas::album_mm;
use super::tl_common::editor_at;
use crate::Command;
use crate::group::ruler_origin;
use crate::new_project::Unit;
use crate::rulers::rulers;

fn close(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-6
}

#[test]
fn eg_b22_regua_marca_zero_no_corte() {
    let project = album_mm(&[(20.0, 40.0, 40.0, 50.0)]);
    let w = project.spec.canvas_rect().width.get();
    let (ox, oy) = ruler_origin(&project);
    let r = rulers(&project, Unit::Cm);
    let zero_x = r.xs.iter().find(|t| t.label == "0").unwrap();
    let zero_y = r.ys.iter().find(|t| t.label == "0").unwrap();
    assert!(
        close(zero_x.at, ox / w) && close(zero_y.at, oy / w),
        "o zero fica no corte"
    );
    // Em cm: uma marca por centímetro e o número a cada 5.
    let five = r.xs.iter().find(|t| t.label == "5").unwrap();
    assert!(close(five.at - zero_x.at, 50.0 / w));
    assert!(
        r.xs.iter()
            .filter(|t| t.major)
            .all(|t| !t.label.is_empty() || t.at < zero_x.at)
    );
    // Em mm, os números contam em mm; na sangria, as marcas não têm número negativo.
    let mm = rulers(&project, Unit::Mm);
    assert!(mm.xs.iter().any(|t| t.label == "50"));
    assert!(mm.xs.iter().all(|t| !t.label.starts_with('-')));
    assert!(mm.xs.first().unwrap().at >= 0.0 && mm.xs.last().unwrap().at <= 1.0 + 1e-9);
}

#[test]
fn eg_paleta_desenha_quadro_e_texto() {
    let project = album_mm(&[(20.0, 40.0, 40.0, 50.0)]);
    let mut editor = editor_at(project, 1);
    assert!(editor.run(Command::Organize));
    let (canvas, trim) = {
        let p = &editor.session().unwrap().project().spec;
        (p.canvas_rect(), p.trim_rect())
    };
    let w = canvas.width.get();
    // De (100, 50) a (160, 90) mm a partir do corte, arrastando de baixo para cima.
    let f = |x: f64, y: f64| {
        (
            (trim.x.get() + x - canvas.x.get()) / w,
            (trim.y.get() + y - canvas.y.get()) / w,
        )
    };
    let before = editor.frames_mm().len();
    assert!(editor.draw_frame(false, f(160.0, 90.0), f(100.0, 50.0)));
    let frames = editor.frames_mm();
    assert_eq!(frames.len(), before + 1);
    let r = frames[before];
    assert!(close(r.x.get() - trim.x.get(), 100.0) && close(r.width.get(), 60.0));
    assert!(close(r.y.get() - trim.y.get(), 50.0) && close(r.height.get(), 40.0));
    assert_eq!(
        editor.selected_frames(),
        vec![before],
        "o quadro novo fica selecionado"
    );
    let s = editor.session().unwrap();
    assert!(s.project().spreads[1].manual);
    assert!(
        s.history_list()
            .steps
            .last()
            .unwrap()
            .label
            .starts_with("Novo quadro")
    );

    assert!(editor.draw_frame(true, f(200.0, 20.0), f(300.0, 40.0)));
    let s = editor.session().unwrap();
    let last = s.project().spreads[1].frames.last().unwrap();
    assert!(matches!(last.content, FrameContent::Text(_)));
    assert!(
        s.history_list()
            .steps
            .last()
            .unwrap()
            .label
            .starts_with("Novo texto")
    );
}
