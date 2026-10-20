//! As vagas da lâmina grande (spec MV, regra 4; ADR 0011, decisão 2): cada quadro mostrado ocupa
//! uma vaga, e a mesma foto continua na mesma vaga entre um estado e outro, para a tela deslizar
//! a foto do lugar antigo ao novo em vez de recriar tudo. As vagas novas entram no fim da lista e
//! as que perderam o quadro esmaecem no lugar até [`Slots::sweep`] retirá-las.

use lamina_core::{FrameId, PhotoId, SpreadId};

use crate::view::{FrameKind, FrameView, PxRect, photo_extent};

/// O que ocupa a vaga.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SlotContent {
    Photo(PhotoId),
    Text,
    Empty,
}

/// Uma vaga: o quadro que ela mostra agora e como chegar lá.
#[derive(Debug, Clone, PartialEq)]
pub struct Slot {
    /// Número da vaga, o mesmo enquanto ela existir.
    pub id: u64,
    /// Posição do quadro na lâmina (`None`: a vaga perdeu o quadro e está saindo).
    pub frame: Option<usize>,
    pub frame_id: Option<FrameId>,
    pub content: SlotContent,
    /// O quadro, em frações da largura da lâmina.
    pub rect: PxRect,
    /// A foto inteira (quadros de foto): a tela a desenha recortada pelo quadro (regra 7).
    pub extent: Option<PxRect>,
    /// Desliza do retângulo antigo para o novo (regras 5 e 8).
    pub fly: bool,
    /// `false`: esmaece no lugar e depois sai.
    pub live: bool,
    /// Nasceu nesta atualização: entra esmaecendo.
    pub born: bool,
}

/// Onde a lâmina mostrada está: o álbum (sobe a cada álbum aberto) e a lâmina.
pub type Place = (u64, SpreadId);

/// As vagas da lâmina grande.
#[derive(Debug, Default)]
pub struct Slots {
    slots: Vec<Slot>,
    next: u64,
    place: Option<Place>,
}

impl Slots {
    /// As vagas, na ordem da lista da tela.
    pub fn slots(&self) -> &[Slot] {
        &self.slots
    }

    /// Leva as vagas aos quadros `frames` da lâmina em `place`. `gesture`: um gesto à mão está
    /// em curso (nada desliza). Devolve `true` quando as vagas recomeçam (outra lâmina, outro
    /// álbum): a tela troca a lista inteira, sem animar.
    pub fn update(&mut self, frames: &[FrameView], place: Place, gesture: bool) -> bool {
        if self.place != Some(place) {
            self.place = Some(place);
            self.slots = frames
                .iter()
                .enumerate()
                .map(|(i, f)| self.fresh(i, f, false))
                .collect();
            return true;
        }
        for s in &mut self.slots {
            s.born = false;
            s.fly = false;
        }
        let pairs = pair(&self.slots, frames);
        let mut taken = vec![false; frames.len()];
        for (_, frame) in &pairs {
            taken[*frame] = true;
        }
        let paired: Vec<usize> = pairs.iter().map(|(s, _)| *s).collect();
        for (i, s) in self.slots.iter_mut().enumerate() {
            if s.live && !paired.contains(&i) {
                s.live = false;
                s.frame = None;
            }
        }
        for (slot, frame) in pairs {
            let new = &frames[frame];
            let s = &mut self.slots[slot];
            let moved = s.rect != new.rect;
            let resized_text = matches!(new.kind, FrameKind::Text { .. })
                && (s.rect.width != new.rect.width || s.rect.height != new.rect.height);
            s.fly = moved && !gesture && !resized_text;
            s.frame = Some(frame);
            s.frame_id = Some(new.id);
            s.content = content(new);
            s.rect = new.rect;
            s.extent = photo_extent(new);
        }
        for (i, f) in frames.iter().enumerate() {
            if !taken[i] {
                let slot = self.fresh(i, f, !gesture);
                self.slots.push(slot);
            }
        }
        false
    }

    /// Retira as vagas que terminaram de esmaecer. Devolve as posições removidas, da maior para a
    /// menor, para a tela remover uma a uma sem mexer nas outras.
    pub fn sweep(&mut self) -> Vec<usize> {
        let dead: Vec<usize> = (0..self.slots.len())
            .rev()
            .filter(|&i| !self.slots[i].live)
            .collect();
        for &i in &dead {
            self.slots.remove(i);
        }
        dead
    }

    fn fresh(&mut self, index: usize, frame: &FrameView, born: bool) -> Slot {
        self.next += 1;
        Slot {
            id: self.next,
            frame: Some(index),
            frame_id: Some(frame.id),
            content: content(frame),
            rect: frame.rect,
            extent: photo_extent(frame),
            fly: false,
            live: true,
            born,
        }
    }
}

fn content(frame: &FrameView) -> SlotContent {
    match &frame.kind {
        FrameKind::Photo { id, .. } => SlotContent::Photo(*id),
        FrameKind::Text { .. } => SlotContent::Text,
        FrameKind::Empty => SlotContent::Empty,
    }
}

/// Os pares (vaga, quadro) pela regra 4: pela foto (a mesma foto duas vezes, pela ordem), depois
/// pelo id do quadro, os textos pela ordem e os quadros vazios pelo mais próximo.
fn pair(slots: &[Slot], frames: &[FrameView]) -> Vec<(usize, usize)> {
    let mut open: Vec<usize> = (0..slots.len()).filter(|&i| slots[i].live).collect();
    open.sort_by_key(|&i| slots[i].frame);
    let mut free: Vec<usize> = (0..frames.len()).collect();
    let mut pairs = Vec::new();
    // 1. A mesma foto, pela ordem.
    for f in free.clone() {
        let SlotContent::Photo(id) = content(&frames[f]) else {
            continue;
        };
        if let Some(k) = open
            .iter()
            .position(|&s| slots[s].content == SlotContent::Photo(id))
        {
            pairs.push((open.remove(k), f));
            free.retain(|&x| x != f);
        }
    }
    // 2. O mesmo quadro (o id não mudou).
    for f in free.clone() {
        if let Some(k) = open
            .iter()
            .position(|&s| slots[s].frame_id == Some(frames[f].id))
        {
            pairs.push((open.remove(k), f));
            free.retain(|&x| x != f);
        }
    }
    // 3. Os textos, pela ordem.
    for f in free.clone() {
        if content(&frames[f]) != SlotContent::Text {
            continue;
        }
        if let Some(k) = open
            .iter()
            .position(|&s| slots[s].content == SlotContent::Text)
        {
            pairs.push((open.remove(k), f));
            free.retain(|&x| x != f);
        }
    }
    // 4. Os vazios, pelo mais próximo.
    for f in free.clone() {
        if content(&frames[f]) != SlotContent::Empty {
            continue;
        }
        let near = open
            .iter()
            .enumerate()
            .filter(|(_, s)| slots[**s].content == SlotContent::Empty)
            .min_by(|a, b| {
                let d = |s: usize| distance(&slots[s].rect, &frames[f].rect);
                d(*a.1).total_cmp(&d(*b.1))
            })
            .map(|(k, _)| k);
        if let Some(k) = near {
            pairs.push((open.remove(k), f));
            free.retain(|&x| x != f);
        }
    }
    pairs
}

/// Distância entre os centros de dois retângulos.
fn distance(a: &PxRect, b: &PxRect) -> f64 {
    let (ax, ay) = (a.x + a.width / 2.0, a.y + a.height / 2.0);
    let (bx, by) = (b.x + b.width / 2.0, b.y + b.height / 2.0);
    (ax - bx).hypot(ay - by)
}
