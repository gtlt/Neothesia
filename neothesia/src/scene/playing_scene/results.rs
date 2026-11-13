use crate::{icons, locals};

use super::midi_player::Score;

/// What the player asked for from the results card
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResultsAction {
    None,
    PlayAgain,
    Listen,
    Menu,
}

/// Shown at the end of a song the player played along: how many notes were right
/// at the first press. Nothing more, the point is to make one want to do better.
pub struct Results {
    pub score: Score,
    /// Score of the attempt before this one, if any
    pub previous: Option<Score>,
}

impl Results {
    /// `area` is the part of the window the card gets centered in.
    pub fn ui(&self, ui: &mut nuon::Ui, area: nuon::Rect) -> ResultsAction {
        // Everything is sized for a 480 pixels tall area, and grows with it
        let u = (area.height() / 480.0).clamp(1.0, 2.0);

        let card_w = 440.0 * u;
        let card_h = if self.previous.is_some() {
            318.0
        } else {
            290.0
        } * u;

        let x = area.min_x() + nuon::center_x(area.width(), card_w);
        let y = area.min_y() + nuon::center_y(area.height(), card_h);

        let mut action = ResultsAction::None;

        // A layer of its own, to be above everything else whatever the order of calls is
        nuon::layer().build(ui, |ui| {
            nuon::translate().pos(x, y).build(ui, |ui| {
                nuon::quad()
                    .size(card_w, card_h)
                    .color([27, 26, 31])
                    .border_radius([16.0 * u; 4])
                    .border_color([58, 56, 66])
                    .border_width(1.0)
                    .build(ui);

                nuon::label()
                    .text(format!("{} / {}", self.score.first_try, self.score.total))
                    .font_size(64.0 * u)
                    .bold(true)
                    .pos(0.0, 26.0 * u)
                    .size(card_w, 72.0 * u)
                    .build(ui);

                nuon::label()
                    .text(locals::t().playing.results_caption)
                    .font_size(17.0 * u)
                    .color([190, 188, 198])
                    .pos(0.0, 102.0 * u)
                    .size(card_w, 24.0 * u)
                    .build(ui);

                let mut cursor = 142.0 * u;

                self.bar(ui, card_w, cursor, u);
                cursor += 26.0 * u;

                if let Some(previous) = self.previous {
                    nuon::label()
                        .text(locals::playing::results_previous(previous.first_try))
                        .font_size(14.0 * u)
                        .color([140, 138, 150])
                        .pos(0.0, cursor)
                        .size(card_w, 20.0 * u)
                        .build(ui);
                    cursor += 28.0 * u;
                }

                cursor += 8.0 * u;

                let buttons = [
                    (
                        ResultsAction::PlayAgain,
                        icons::replay_icon(),
                        locals::t().playing.play_again,
                    ),
                    (
                        ResultsAction::Listen,
                        icons::ear_icon(),
                        locals::t().playing.listen,
                    ),
                    (
                        ResultsAction::Menu,
                        icons::left_arrow_icon(),
                        locals::t().playing.menu,
                    ),
                ];

                let btn_w = 104.0 * u;
                let btn_h = 60.0 * u;
                let gap = 16.0 * u;
                let row_w = btn_w * buttons.len() as f32 + gap * (buttons.len() - 1) as f32;

                nuon::translate()
                    .pos(nuon::center_x(card_w, row_w), cursor)
                    .build(ui, |ui| {
                        for (id, (button_action, icon, caption)) in buttons.into_iter().enumerate()
                        {
                            let is_main = button_action == ResultsAction::PlayAgain;
                            if button(ui, id, btn_w, btn_h, u, icon, caption, is_main) {
                                action = button_action;
                            }

                            nuon::translate().x(btn_w + gap).add_to_current(ui);
                        }
                    });
            });
        });

        action
    }

    /// Share of the notes that were right at once
    fn bar(&self, ui: &mut nuon::Ui, card_w: f32, y: f32, u: f32) {
        let w = 300.0 * u;
        let h = 10.0 * u;
        let x = nuon::center_x(card_w, w);

        nuon::quad()
            .pos(x, y)
            .size(w, h)
            .color([58, 56, 66])
            .border_radius([h / 2.0; 4])
            .build(ui);

        if self.score.total == 0 || self.score.first_try == 0 {
            return;
        }

        let share = self.score.first_try as f32 / self.score.total as f32;

        nuon::quad()
            .pos(x, y)
            // Never narrower than it is tall, to keep its round ends
            .size((w * share.min(1.0)).max(h), h)
            .color([86, 186, 120])
            .border_radius([h / 2.0; 4])
            .build(ui);
    }
}

#[allow(clippy::too_many_arguments)]
fn button(
    ui: &mut nuon::Ui,
    id: usize,
    w: f32,
    h: f32,
    u: f32,
    icon: &str,
    caption: &'static str,
    is_main: bool,
) -> bool {
    let event = nuon::click_area(nuon::Id::hash(("results_button", id)))
        .size(w, h)
        .build(ui);

    let is_hot = event.is_hovered() || event.is_pressed();

    let color: [u8; 3] = match (is_main, is_hot) {
        (true, false) => [56, 124, 224],
        (true, true) => [76, 144, 244],
        (false, false) => [47, 45, 54],
        (false, true) => [64, 62, 72],
    };

    nuon::quad()
        .size(w, h)
        .color(color)
        .border_radius([10.0 * u; 4])
        .build(ui);

    nuon::label()
        .icon(icon)
        .font_size(26.0 * u)
        .size(w, h)
        .build(ui);

    nuon::label()
        .text(caption)
        .font_size(13.0 * u)
        .color([190, 188, 198])
        .pos(0.0, h + 6.0 * u)
        .size(w, 18.0 * u)
        .build(ui);

    event.is_clicked()
}
