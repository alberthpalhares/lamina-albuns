//! Toca a animação da novidade escolhida no "O que há de novo" (spec NV, regra 12). O que cada
//! quadro mostra e quando o laço volta sai do `lamina_editor::updates::news::Animation`; aqui só o
//! temporizador e a troca da imagem na janela.

use std::cell::RefCell;
use std::time::Duration;

use lamina_editor::updates::news::{Animation, Illustration};
use slint::{ComponentHandle, Image, Rgba8Pixel, SharedPixelBuffer};

use crate::{AppWindow, NewsUi};

/// As ilustrações da lista, a animação que toca e a geração dela: escolher outra novidade muda a
/// geração, e o quadro agendado da anterior não troca mais nada.
#[derive(Default)]
struct Player {
    entries: Vec<Option<&'static [u8]>>,
    reduce_motion: bool,
    generation: u64,
    animation: Option<Animation<'static>>,
}

thread_local! {
    static PLAYER: RefCell<Player> = RefCell::new(Player::default());
}

/// A imagem da janela a partir de um quadro decodificado. Uma cópia só, de bytes: a cópia pixel a
/// pixel levava ~8 ms por quadro no Lamina compilado em debug.
pub fn image(picture: &Illustration) -> Image {
    let mut buffer = SharedPixelBuffer::<Rgba8Pixel>::new(picture.width, picture.height);
    let bytes = buffer.make_mut_bytes();
    if bytes.len() != picture.rgba.len() {
        return Image::default();
    }
    bytes.copy_from_slice(&picture.rgba);
    Image::from_rgba8(buffer)
}

/// Guarda as ilustrações de cada linha da lista (na ordem da lista) ao abrir a janela.
pub fn load(entries: Vec<Option<&'static [u8]>>, reduce_motion: bool) {
    PLAYER.with_borrow_mut(|p| {
        p.entries = entries;
        p.reduce_motion = reduce_motion;
    });
}

/// Começa a tocar a ilustração da linha `index`. Parada, sem ilustração ou com "Reduzir
/// movimento", só para a anterior: a janela mostra o primeiro quadro, que já está na lista.
pub fn choose(window: &AppWindow, index: usize) {
    window.global::<NewsUi>().set_playing(false);
    let delay = PLAYER.with_borrow_mut(|p| {
        p.generation += 1;
        let bytes = p.entries.get(index).copied().flatten();
        p.animation = bytes
            .and_then(|b| Animation::open(b, p.reduce_motion))
            .filter(Animation::animated);
        p.animation
            .as_ref()
            .map(|a| (p.generation, a.first().delay_ms))
    });
    if let Some((generation, delay_ms)) = delay {
        schedule(window.as_weak(), generation, delay_ms);
    }
}

/// Para de tocar (a janela fechou).
pub fn stop() {
    PLAYER.with_borrow_mut(|p| {
        p.generation += 1;
        p.animation = None;
    });
}

fn schedule(window: slint::Weak<AppWindow>, generation: u64, delay_ms: u32) {
    slint::Timer::single_shot(Duration::from_millis(u64::from(delay_ms)), move || {
        tick(&window, generation);
    });
}

/// Troca para o quadro seguinte e agenda o próximo; fechar a janela para o laço.
fn tick(window: &slint::Weak<AppWindow>, generation: u64) {
    let Some(w) = window.upgrade() else {
        return stop();
    };
    let ui = w.global::<NewsUi>();
    if !ui.get_show() {
        return stop();
    }
    let frame = PLAYER.with_borrow_mut(|p| {
        if p.generation != generation {
            return None;
        }
        p.animation.as_mut().and_then(Animation::advance)
    });
    let Some(frame) = frame else {
        return;
    };
    ui.set_frame(image(&frame.picture));
    ui.set_playing(true);
    schedule(window.clone(), generation, frame.delay_ms);
}
