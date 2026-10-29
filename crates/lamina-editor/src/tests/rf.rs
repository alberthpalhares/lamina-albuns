//! Cenários da spec `reencontrar-fotos` (RF): as pastas ausentes, a pasta que mudou de lugar, a
//! foto localizada com as vizinhas, o disco desconectado e a IA. Tudo sem janela.

use std::cell::Cell;
use std::path::{Path, PathBuf};

use lamina_core::{PhotoId, Project};
use serde_json::json;

use super::common::Base;
use super::ia_common::Lab;
use crate::relink_moved::take_reads;
use crate::relink_text::{PLUG_IN, relink_view};
use crate::{Editor, Located, Session, locate, missing_folders, search_moved, still_missing};

/// `n` fotos em `fotos/Casamento` (as duas primeiras em `fotos/Casamento/Making of`), num álbum
/// gravado em `album/` (longe das fotos, como um álbum num disco e as fotos noutro).
fn wedding(n: usize) -> (Base, PathBuf) {
    let base = Base::with_photos(n, (300, 200));
    let wedding = base.path("fotos/Casamento");
    std::fs::create_dir_all(wedding.join("Making of")).unwrap();
    for i in 0..n {
        let name = format!("f{i:02}.jpg");
        let to = if i < 2 {
            wedding.join("Making of").join(&name)
        } else {
            wedding.join(&name)
        };
        std::fs::rename(base.path(&format!("fotos/{name}")), to).unwrap();
    }
    let file = base.save(base.project(2), "album/album.lamina");
    (base, file)
}

fn opened(file: &Path) -> Editor {
    let mut editor = Editor::new();
    editor.open(file).unwrap();
    editor
}

fn session(editor: &mut Editor) -> &mut Session {
    editor.session_mut().unwrap()
}

/// Move `from` para `to` (a pasta de cima é criada).
fn move_dir(from: &Path, to: &Path) {
    std::fs::create_dir_all(to.parent().unwrap()).unwrap();
    std::fs::rename(from, to).unwrap();
}

/// Fotos que não são do álbum na pasta, para mostrar que elas não são lidas.
fn strangers(dir: &Path, n: usize) {
    let other = Base::with_photos(n, (120, 80));
    for i in 0..n {
        let name = format!("outra-{i:02}.jpg");
        std::fs::copy(other.path(&format!("fotos/f{i:02}.jpg")), dir.join(name)).unwrap();
    }
}

fn relink_in(editor: &mut Editor, from: Option<&Path>, to: &Path) -> usize {
    let s = session(editor);
    let found = search_moved(s.project(), s.missing(), from, to, &|_, _| {});
    s.relink(&found)
}

fn label(editor: &Editor) -> String {
    let list = editor.session().unwrap().history_list();
    list.steps.last().unwrap().label.clone()
}

#[test]
fn rf_b1_as_fotos_ausentes_agrupadas_por_pasta() {
    let mut project = Project::new("teste", super::tl_common::spec(30.0, 30.0));
    let root = std::env::temp_dir().join("lamina-rf-b1-nao-existe");
    let mut ids: Vec<PhotoId> = Vec::new();
    for (dir, n) in [("Casamento", 6), ("Casamento/Making of", 2), ("Ensaio", 3)] {
        for i in 0..n {
            ids.push(project.add_photo(root.join(dir).join(format!("{i}.jpg")), 300, 200));
        }
    }
    let folders = missing_folders(&project, &ids);
    let shown: Vec<(String, usize)> = folders.iter().map(|f| (f.name(), f.count)).collect();
    assert_eq!(
        shown,
        [("Casamento".to_owned(), 8), ("Ensaio".to_owned(), 3)]
    );
    assert_eq!(
        folders[0].detail(),
        format!("Casamento · 8 fotos · estava em {}", root.display())
    );
}

#[test]
fn rf_b2_a_janela_abre_so_quando_ainda_faltam_fotos() {
    let (base, file) = wedding(4);
    let mut fine = opened(&file);
    assert!(!session(&mut fine).take_relink_prompt());
    move_dir(&base.path("fotos/Casamento"), &base.path("ssd/Casamento"));
    let mut editor = opened(&file);
    assert!(session(&mut editor).take_relink_prompt());
    assert!(!session(&mut editor).take_relink_prompt(), "uma vez só");
    let view = relink_view(editor.session().unwrap()).unwrap();
    assert_eq!(view.folders.len(), 1);
    assert!(!view.unplugged);
}

#[test]
fn rf_b3_a_pasta_inteira_levada_para_outro_disco() {
    let (base, file) = wedding(8);
    let new = base.path("ssd/Casamento");
    move_dir(&base.path("fotos/Casamento"), &new);
    strangers(&new, 20);
    let mut editor = opened(&file);
    assert_eq!(editor.session().unwrap().missing().len(), 8);
    take_reads();
    assert_eq!(
        relink_in(&mut editor, Some(&base.path("fotos/Casamento")), &new),
        8
    );
    assert_eq!(take_reads(), 8, "só os candidatos são lidos");
    assert!(editor.session().unwrap().missing().is_empty());
    assert_eq!(label(&editor), "Reencontrar fotos · 8 fotos");
    assert!(editor.run(crate::Command::Undo));
    assert_eq!(editor.session().unwrap().missing().len(), 8);
}

#[test]
fn rf_b4_escolher_a_pasta_de_cima_tambem_serve() {
    let (base, file) = wedding(8);
    move_dir(
        &base.path("fotos/Casamento"),
        &base.path("ssd/Trabalhos 2026/Casamento"),
    );
    let mut editor = opened(&file);
    let parent = base.path("ssd/Trabalhos 2026");
    let from = base.path("fotos/Casamento");
    assert_eq!(relink_in(&mut editor, Some(&from), &parent), 8);
}

#[test]
fn rf_b5_arquivos_renomeados_sao_achados_pelo_conteudo() {
    let (base, file) = wedding(4);
    let new = base.path("ssd/novo");
    std::fs::create_dir_all(&new).unwrap();
    for path in crate::relink::index_by_name(&base.path("fotos/Casamento"))
        .into_values()
        .flatten()
    {
        let name = format!("renomeada-{}", path.file_name().unwrap().to_string_lossy());
        std::fs::rename(&path, new.join(name)).unwrap();
    }
    let mut editor = opened(&file);
    let s = session(&mut editor);
    let steps = Cell::new(0);
    let from = base.path("fotos/Casamento");
    let found = search_moved(s.project(), s.missing(), Some(&from), &new, &|_, _| {
        steps.set(steps.get() + 1);
    });
    assert_eq!(found.len(), 4);
    assert!(steps.get() >= 4, "com andamento");
}

#[test]
fn rf_b6_uma_escolha_resolve_as_outras_pastas_no_mesmo_disco() {
    let base = Base::with_photos(6, (300, 200));
    for (dir, range) in [("Casamento", 0..3), ("Ensaio", 3..6)] {
        std::fs::create_dir_all(base.path(&format!("fotos/{dir}"))).unwrap();
        for i in range {
            let name = format!("f{i:02}.jpg");
            let to = base.path(&format!("fotos/{dir}/{name}"));
            std::fs::rename(base.path(&format!("fotos/{name}")), to).unwrap();
        }
    }
    let file = base.save(base.project(2), "album/album.lamina");
    move_dir(&base.path("fotos"), &base.path("ssd/fotos"));
    let mut editor = opened(&file);
    assert_eq!(editor.session().unwrap().missing().len(), 6);
    let from = base.path("fotos/Casamento");
    take_reads();
    assert_eq!(
        relink_in(&mut editor, Some(&from), &base.path("ssd/fotos/Casamento")),
        6
    );
    assert_eq!(take_reads(), 6);
    assert!(
        relink_view(editor.session().unwrap()).is_none(),
        "não falta nada"
    );
}

#[test]
fn rf_b7_localizar_uma_foto_traz_as_vizinhas() {
    let (base, file) = wedding(8);
    let new = base.path("ssd/Casamento");
    move_dir(&base.path("fotos/Casamento"), &new);
    let mut editor = opened(&file);
    let s = session(&mut editor);
    let id = s.missing()[3];
    let name = s
        .project()
        .photo(id)
        .unwrap()
        .path
        .file_name()
        .unwrap()
        .to_owned();
    let picked =
        crate::relink::index_by_name(&new)[&name.to_string_lossy().into_owned()][0].clone();
    let Located::Same(found) = locate(s.project(), s.missing(), id, &picked).unwrap() else {
        panic!("é a mesma foto");
    };
    assert_eq!(s.relink(&found), 8);
    assert!(s.missing().is_empty());
}

#[test]
fn rf_b8_um_arquivo_com_outro_conteudo_pede_confirmacao() {
    let (base, file) = wedding(3);
    std::fs::remove_file(base.path("fotos/Casamento/f02.jpg")).unwrap();
    let mut editor = opened(&file);
    let s = session(&mut editor);
    let id = s.missing()[0];
    let other = base.path("fotos/Casamento/Making of/f01.jpg");
    let before = s.project().photo(id).unwrap().fingerprint.clone();
    let Located::Different(found) = locate(s.project(), s.missing(), id, &other).unwrap() else {
        panic!("é outro conteúdo");
    };
    // Cancelar: nada muda.
    assert_eq!(s.project().photo(id).unwrap().fingerprint, before);
    assert_eq!(s.relink(std::slice::from_ref(&found)), 1);
    let photo = s.project().photo(id).unwrap();
    assert_eq!(photo.path, other);
    assert_ne!(photo.fingerprint, before);
    assert!(s.missing().is_empty());
    assert!(crate::relink_text::DIFFERENT.starts_with("Este arquivo não é a mesma foto"));
}

/// Um caminho num disco que não existe nesta máquina.
fn unplugged_root() -> PathBuf {
    if cfg!(windows) {
        let letter = ('D'..='Z')
            .rev()
            .find(|l| !Path::new(&format!("{l}:\\")).exists())
            .unwrap();
        PathBuf::from(format!("{letter}:\\Fotos"))
    } else {
        PathBuf::from("/media/lamina-teste/SSD-RF/Fotos")
    }
}

#[test]
fn rf_b9_disco_desconectado() {
    let mut project = Project::new("teste", super::tl_common::spec(30.0, 30.0));
    let root = unplugged_root();
    let id = project.add_photo(root.join("Casamento").join("a.jpg"), 300, 200);
    let folders = missing_folders(&project, &[id]);
    let disk = folders[0].disk.clone().unwrap();
    assert!(
        folders[0]
            .detail()
            .ends_with(&format!("no disco {disk}, que não está conectado"))
    );
    let session = Session::new_unsaved(project);
    let view = relink_view(&session).unwrap();
    assert!(view.unplugged);
    assert!(PLUG_IN.contains("conecte o disco e clique em Tentar de novo"));
}

#[test]
fn rf_b10_conectar_o_disco_e_tentar_de_novo_nao_vira_edicao() {
    let (base, file) = wedding(4);
    let old = base.path("fotos/Casamento");
    move_dir(&old, &base.path("guardada"));
    let mut editor = opened(&file);
    let steps = editor.session().unwrap().history_list().steps.len();
    assert_eq!(editor.session().unwrap().missing().len(), 4);
    // O "disco" volta: os arquivos nos mesmos caminhos.
    move_dir(&base.path("guardada"), &old);
    let still = still_missing(editor.session().unwrap().project());
    assert!(session(&mut editor).checked_missing(&still));
    let s = editor.session().unwrap();
    assert!(s.missing().is_empty());
    assert!(!s.is_dirty());
    assert_eq!(s.history_list().steps.len(), steps);
}

#[test]
fn rf_b11_a_ia_usa_a_mesma_procura() {
    let (base, file) = wedding(8);
    let new = base.path("ssd/Casamento");
    move_dir(&base.path("fotos/Casamento"), &new);
    strangers(&new, 20);
    let mut lab = Lab::new(&base, vec![base.dir.path().to_path_buf()], false);
    lab.ok("abrir_album", json!({ "caminho": file }));
    take_reads();
    let reply = lab.ok(
        "reencontrar_fotos",
        json!({ "pasta": new.display().to_string() }),
    );
    assert_eq!(reply.data["reencontradas"], 8);
    assert_eq!(take_reads(), 8);
}

#[test]
fn rf_b7_duplo_clique_numa_foto_ausente_pede_para_localizar() {
    let (base, file) = wedding(4);
    move_dir(&base.path("fotos/Casamento"), &base.path("ssd/Casamento"));
    let mut editor = opened(&file);
    let photo = editor.missing_photo_at(0).unwrap();
    assert_eq!(
        editor.double_click(Some(0)),
        crate::editor::DoubleClick::Locate(photo)
    );
    assert!(editor.available(crate::Command::LocatePhoto));
    let menu = editor.context_menu(Some(0));
    assert!(matches!(
        menu.first(),
        Some(crate::editor::ContextEntry::Item {
            command: crate::Command::LocatePhoto,
            ..
        })
    ));
}
