//! A atualização da barra do projeto, dos comandos, da lâmina atual e dos painéis (saiu do
//! `state.rs`; auditoria do Marco 3, M10).

use lamina_editor::forms::album_info;
use lamina_editor::{Command, strip_titles};

use crate::state::App;

impl App {
    /// Título, contador, comandos disponíveis e a lâmina atual com o painel dela.
    pub fn refresh_current(&mut self) {
        let Some(w) = self.window.upgrade() else {
            return;
        };
        w.set_has_project(self.session().is_some());
        w.set_can_save(self.editor.available(Command::Save));
        w.set_has_missing(self.editor.available(Command::FindMissing));
        let strips = self.editor.strips();
        w.set_spreads_open(strips.spreads);
        w.set_timeline_open(strips.timeline);
        w.set_photos_open(strips.photos);
        w.set_panel_open(strips.panel);
        w.set_can_prev(self.editor.available(Command::PreviousSpread));
        w.set_can_next(self.editor.available(Command::NextSpread));
        w.set_guides(self.editor.guides());
        crate::motion::refresh(&w, self.editor.durations(), self.editor.reduce_motion());
        // O aviso do editor vale também sem álbum: a lista de recentes ilegível aparece na tela
        // inicial (spec TI, regra 9).
        if let Some(notice) = self.editor.take_notice() {
            self.notice = notice;
        }
        w.set_notice(self.notice.clone().into());
        let Some(session) = self.editor.session() else {
            w.set_window_title("Lamina".into());
            w.set_project_title("".into());
            w.set_counter("".into());
            return;
        };
        let dirty = if session.is_dirty() { "• " } else { "" };
        let title = format!("{dirty}{}", session.title());
        w.set_window_title(format!("{title} — Lamina").into());
        w.set_project_title(title.into());
        w.set_counter(session.counter().into());
        w.set_counter_warning(session.page_warning().unwrap_or_default().into());
        // "Lâminas · 4 lâminas (8 páginas)": o nome fica no cabeçalho, aqui vai só o resumo.
        let [spreads, timeline, photos] = strip_titles(session)
            .map(|t| t.split_once(" · ").map_or(t.clone(), |(_, s)| s.to_owned()));
        w.set_strip_spreads(spreads.into());
        w.set_strip_timeline(timeline.into());
        w.set_strip_photos(photos.into());
        let index = session.current();
        let warnings: Vec<String> = session
            .warnings_for(index)
            .iter()
            .map(ToString::to_string)
            .collect();
        let project = session.project().clone();
        let count = session.spread_count();
        let frames = project.spreads.get(index).map_or(0, |s| s.frames.len());
        w.set_info_album(album_info(&project).into());
        let manual = project.spreads.get(index).is_some_and(|s| s.manual);
        w.set_info_spread(lamina_editor::naming::spread_line(index, count, frames, manual).into());
        w.set_info_warnings(warnings.join("\n").into());

        crate::crop::refresh_spread(self);
        if let Some(notice) = self.editor.take_notice() {
            self.notice = notice;
            w.set_notice(self.notice.clone().into());
        }
        crate::panel::refresh(self);
        crate::look::refresh(self);
        crate::history::refresh(self);
        crate::frame_panel::refresh(self);
        crate::details::refresh(self);
    }
}
