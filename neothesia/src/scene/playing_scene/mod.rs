use midi_file::midly::MidiMessage;
use neothesia_core::render::{
    GlowRenderer, GuidelineRenderer, NoteLabels, QuadRenderer, TextRenderer,
};
use std::time::{Duration, Instant};
use winit::{
    event::WindowEvent,
    keyboard::{Key, NamedKey},
};

use self::top_bar::TopBar;

use super::{NuonRenderer, Scene};
use crate::{
    NeothesiaEvent, context::Context, icons, locals, render::WaterfallRenderer,
    scene::MouseToMidiEventState, song::Song, utils::window::WinitEvent,
};

mod keyboard;
pub use keyboard::Keyboard;

pub(crate) mod midi_player;
use midi_player::MidiPlayer;

mod results;
use results::{Results, ResultsAction};

mod rewind_controller;
use rewind_controller::RewindController;

mod toast_manager;
use toast_manager::ToastManager;

mod animation;
mod top_bar;

pub struct PlayingScene {
    keyboard: Keyboard,
    waterfall: WaterfallRenderer,
    guidelines: GuidelineRenderer,
    text_renderer: TextRenderer,
    nuon_renderer: NuonRenderer,

    note_labels: Option<NoteLabels>,

    player: MidiPlayer,
    rewind_controller: RewindController,
    quad_renderer_bg: QuadRenderer,
    glow: Option<GlowRenderer>,
    toast_manager: ToastManager,

    nuon: nuon::Ui,
    mouse_to_midi_state: MouseToMidiEventState,

    deduced_chord_name: String,

    top_bar: TopBar,

    /// Shown once the song is over
    results: Option<Results>,
    /// What the player scored the time before
    last_score: Option<midi_player::Score>,
    banner: Option<Banner>,
}

/// A word in the middle of the screen, to tell who is playing
struct Banner {
    icon: &'static str,
    text: &'static str,
    /// Stays for as long as it is true when there is no end
    until: Option<Instant>,
}

impl Banner {
    fn listening() -> Self {
        Self {
            icon: icons::ear_icon(),
            text: locals::t().playing.listening,
            until: None,
        }
    }

    fn your_turn() -> Self {
        Self {
            icon: icons::note_list_icon(),
            text: locals::t().playing.your_turn,
            until: Some(Instant::now() + Duration::from_millis(2500)),
        }
    }

    fn is_over(&self) -> bool {
        self.until.is_some_and(|until| Instant::now() >= until)
    }
}

impl PlayingScene {
    pub fn new(ctx: &mut Context, song: Song) -> Self {
        let keyboard = Keyboard::new(ctx, song.config.clone());

        let keyboard_layout = keyboard.layout();

        let guidelines = GuidelineRenderer::new(
            keyboard_layout.clone(),
            *keyboard.pos(),
            ctx.config.vertical_guidelines(),
            ctx.config.horizontal_guidelines(),
            song.file.measures.clone(),
        );

        let keyboard_placement = keyboard.placement().clone();

        let hidden_tracks: Vec<usize> = song
            .config
            .tracks
            .iter()
            .filter(|t| !t.visible)
            .map(|t| t.track_id)
            .collect();

        let mut waterfall = WaterfallRenderer::new(
            &ctx.gpu,
            &song.file.tracks,
            &hidden_tracks,
            &ctx.config,
            &ctx.transform,
            &keyboard_placement,
        );

        let text_renderer = ctx.text_renderer_factory.new_renderer();

        let note_labels = ctx.config.note_labels().then_some(NoteLabels::new(
            *keyboard.pos(),
            waterfall.notes(),
            ctx.config.waterfall_note_naming(),
            ctx.text_renderer_factory.new_renderer(),
        ));

        let player = MidiPlayer::new(
            ctx.output_manager.connection().clone(),
            song,
            keyboard_layout.range.clone(),
            ctx.config.separate_channels(),
        );
        waterfall.update(player.time_without_lead_in());

        let quad_renderer_bg = ctx.quad_renderer_factory.new_renderer();

        let glow = ctx.config.glow().then_some(GlowRenderer::new(
            &ctx.gpu,
            &ctx.transform,
            keyboard.layout(),
        ));

        Self {
            keyboard,
            guidelines,
            note_labels,
            text_renderer,
            nuon_renderer: NuonRenderer::new(ctx),

            waterfall,
            player,
            rewind_controller: RewindController::new(),
            quad_renderer_bg,
            glow,
            toast_manager: ToastManager::default(),

            nuon: nuon::Ui::new(),
            mouse_to_midi_state: MouseToMidiEventState::default(),
            deduced_chord_name: String::new(),

            top_bar: TopBar::new(),

            results: None,
            last_score: None,
            banner: None,
        }
    }

    /// Back to the start of the song, with a blank score
    fn restart(&mut self) {
        self.player.restart();
        self.keyboard.reset_notes();
        self.results = None;
    }

    /// While listening the app plays the tracks of the player too.
    /// Either way the song starts over: one listens to all of it, then plays all of it.
    fn set_listening(&mut self, listening: bool) {
        self.player.set_listening(listening);
        self.keyboard.set_listening(listening);
        self.restart();

        self.banner = Some(if listening {
            Banner::listening()
        } else {
            Banner::your_turn()
        });
    }

    fn toggle_listening(&mut self) {
        self.set_listening(!self.player.is_listening());
    }

    fn go_to_menu(&self, ctx: &Context) {
        ctx.proxy
            .send_event(NeothesiaEvent::MainMenu(Some(self.player.song().clone())))
            .ok();
    }

    /// The song reached its end
    fn song_finished(&mut self, ctx: &Context) {
        if self.player.is_listening() {
            // Heard it, now play it
            self.set_listening(false);
        } else if self.player.has_targets() {
            self.player.pause();

            let score = self.player.score();
            self.results = Some(Results {
                score,
                previous: self.last_score.replace(score),
            });
        } else {
            self.go_to_menu(ctx);
        }
    }

    fn results_ui(&mut self, ctx: &Context) {
        // Seeking back into the song takes the card away
        if !self.player.is_finished() {
            self.results = None;
        }

        let Some(results) = &self.results else {
            return;
        };

        let area = nuon::Rect::new(
            nuon::Point::zero(),
            nuon::Size::new(ctx.window_state.logical_size.width, self.keyboard.pos().y),
        );

        match results.ui(&mut self.nuon, area) {
            ResultsAction::None => {}
            ResultsAction::PlayAgain => self.restart(),
            ResultsAction::Listen => self.set_listening(true),
            ResultsAction::Menu => self.go_to_menu(ctx),
        }
    }

    fn banner_ui(&mut self, ctx: &Context) {
        if self.banner.as_ref().is_some_and(Banner::is_over) || self.results.is_some() {
            self.banner = None;
        }

        let Some(banner) = &self.banner else {
            return;
        };

        let w = ctx.window_state.logical_size.width;
        let u = (self.keyboard.pos().y / 480.0).clamp(1.0, 2.0);
        // Under the top bar, even when it is open
        let y = 95.0;

        nuon::label()
            .icon(banner.icon)
            .font_size(30.0 * u)
            .color([225, 224, 230])
            .pos(0.0, y)
            .size(w, 36.0 * u)
            .build(&mut self.nuon);

        nuon::label()
            .text(banner.text)
            .font_size(24.0 * u)
            .color([225, 224, 230])
            .pos(0.0, y + 40.0 * u)
            .size(w, 30.0 * u)
            .build(&mut self.nuon);
    }

    fn update_glow(&mut self, delta: Duration) {
        let Some(glow) = &mut self.glow else {
            return;
        };

        glow.clear();

        let keys = &self.keyboard.layout().keys;
        let states = self.keyboard.key_states();
        let pos = self.keyboard.pos();

        for (key, state) in keys.iter().zip(states) {
            let Some(color) = state.pressed_by_file() else {
                continue;
            };

            glow.push(key.id(), *color, pos.x + key.x(), pos.y, key.width(), delta);
        }
    }

    fn update_chord_identifier(&mut self, enabled: bool) {
        if !enabled {
            return;
        }

        let start = self.keyboard.layout().range.start();
        let notes = self
            .keyboard
            .key_states()
            .iter()
            .enumerate()
            .filter(|(_, state)| state.pressed_by_user().is_some())
            .map(|(id, _)| id as u8 + start)
            .collect::<Vec<_>>();

        self.deduced_chord_name = super::freeplay::chords::deduce_name(&notes).unwrap_or_default();
    }

    #[profiling::function]
    fn update_midi_player(&mut self, ctx: &Context, delta: Duration) -> f32 {
        if self.top_bar.is_looper_active() && self.player.time() > self.top_bar.loop_end_timestamp()
        {
            self.player.set_time(self.top_bar.loop_start_timestamp());
            self.keyboard.reset_notes();
        }

        if self.player.play_along().are_required_keys_pressed() {
            let delta = (delta / 10) * (ctx.config.speed_multiplier() * 10.0) as u32;
            let midi_events = self.player.update(delta);
            self.keyboard.file_midi_events(&ctx.config, &midi_events);
        }

        self.keyboard
            .set_awaited_notes(self.player.play_along().awaited_notes());

        self.player.time_without_lead_in() + ctx.config.animation_offset()
    }

    #[profiling::function]
    fn resize(&mut self, ctx: &mut Context) {
        self.keyboard.resize(ctx);

        self.guidelines.set_layout(self.keyboard.layout().clone());
        self.guidelines.set_pos(*self.keyboard.pos());
        if let Some(note_labels) = self.note_labels.as_mut() {
            note_labels.set_pos(*self.keyboard.pos());
        }

        self.waterfall
            .resize(&ctx.config, self.keyboard.placement());
    }
}

impl Scene for PlayingScene {
    #[profiling::function]
    fn update(&mut self, ctx: &mut Context, delta: Duration) {
        self.quad_renderer_bg.clear();

        self.rewind_controller.update(&mut self.player, ctx, delta);
        self.toast_manager.update(&mut self.text_renderer);

        let time = self.update_midi_player(ctx, delta);
        self.waterfall.update(time);
        self.guidelines.update(
            &mut self.quad_renderer_bg,
            ctx.config.animation_speed(),
            ctx.window_state.scale_factor as f32,
            time,
        );
        self.keyboard.update(ctx, delta);
        self.update_chord_identifier(ctx.config.chord_identifier());
        if let Some(note_labels) = self.note_labels.as_mut() {
            note_labels.update(
                ctx.window_state.physical_size,
                ctx.window_state.scale_factor as f32,
                self.keyboard.renderer(),
                ctx.config.animation_speed(),
                time,
            );
        }

        self.update_glow(delta);

        TopBar::update(self, ctx);
        self.banner_ui(ctx);
        self.results_ui(ctx);

        if ctx.config.chord_identifier() {
            nuon::label()
                .text(&self.deduced_chord_name)
                .font_size(25.0)
                .y(self.keyboard.pos().y - 35.0)
                .height(25.0)
                .width(ctx.window_state.logical_size.width)
                .build(&mut self.nuon);
        }

        super::render_nuon(&mut self.nuon, &mut self.nuon_renderer, ctx);

        self.quad_renderer_bg.prepare();

        if let Some(glow) = &mut self.glow {
            glow.prepare();
        }

        #[cfg(debug_assertions)]
        self.text_renderer.queue_fps(
            ctx.fps_ticker.avg(),
            self.top_bar
                .topbar_expand_animation
                .animate_bool(5.0, 80.0, ctx.frame_timestamp),
        );
        self.text_renderer.update(
            ctx.window_state.physical_size,
            ctx.window_state.scale_factor as f32,
        );

        if self.player.is_finished() && !self.player.is_paused() {
            self.song_finished(ctx);
        }
    }

    #[profiling::function]
    fn render<'pass>(&'pass mut self, rpass: &mut wgpu_jumpstart::RenderPass<'pass>) {
        self.quad_renderer_bg.render(rpass);
        self.waterfall.render(rpass);
        if let Some(note_labels) = self.note_labels.as_mut() {
            note_labels.render(rpass);
        }
        self.keyboard.render(rpass);
        if let Some(glow) = &self.glow {
            glow.render(rpass);
        }
        self.text_renderer.render(rpass);

        self.nuon_renderer.render(rpass);
    }

    fn window_event(&mut self, ctx: &mut Context, event: &WindowEvent) {
        self.rewind_controller
            .handle_window_event(ctx, event, &mut self.player);

        if self.rewind_controller.is_rewinding() {
            self.keyboard.reset_notes();
        }

        if event.back_mouse_pressed() || event.key_released(Key::Named(NamedKey::Escape)) {
            self.go_to_menu(ctx);
        }

        if self.results.is_some() {
            // The song is over: the keys that would resume it start it again
            if event.key_released(Key::Named(NamedKey::Space))
                || event.key_released(Key::Named(NamedKey::Enter))
            {
                self.restart();
            }
        } else if event.key_released(Key::Named(NamedKey::Space)) {
            self.player.pause_resume();
        }

        if event.key_released(Key::Named(NamedKey::Home)) {
            self.restart();
        }

        if event.key_released(Key::Named(NamedKey::Tab)) && self.player.has_targets() {
            self.toggle_listening();
        }

        handle_settings_input(ctx, &mut self.toast_manager, &mut self.waterfall, event);
        super::handle_pc_keyboard_to_midi_event(ctx, event);
        super::handle_mouse_to_midi_event(
            &mut self.keyboard,
            &mut self.mouse_to_midi_state,
            ctx,
            event,
        );

        if event.window_resized() || event.scale_factor_changed() {
            self.resize(ctx)
        }

        super::handle_nuon_window_event(&mut self.nuon, event, ctx);
    }

    fn midi_event(&mut self, ctx: &mut Context, channel: u8, message: &MidiMessage) {
        let verdict = self.player.user_midi_event(channel, message);
        self.keyboard
            .judged_user_midi_event(&ctx.config, message, verdict);
    }
}

fn handle_settings_input(
    ctx: &mut Context,
    toast_manager: &mut ToastManager,
    waterfall: &mut WaterfallRenderer,
    event: &WindowEvent,
) {
    if event.key_released(Key::Named(NamedKey::ArrowUp))
        || event.key_released(Key::Named(NamedKey::ArrowDown))
    {
        let amount = if ctx.window_state.modifiers_state.shift_key() {
            0.5
        } else {
            0.1
        };

        if event.key_released(Key::Named(NamedKey::ArrowUp)) {
            ctx.config
                .set_speed_multiplier(ctx.config.speed_multiplier() + amount);
        } else {
            ctx.config
                .set_speed_multiplier(ctx.config.speed_multiplier() - amount);
        }

        toast_manager.speed_toast(ctx.config.speed_multiplier());
        return;
    }

    if event.key_released(Key::Named(NamedKey::PageUp))
        || event.key_released(Key::Named(NamedKey::PageDown))
    {
        let amount = if ctx.window_state.modifiers_state.shift_key() {
            500.0
        } else {
            100.0
        };

        if event.key_released(Key::Named(NamedKey::PageUp)) {
            ctx.config
                .set_animation_speed(ctx.config.animation_speed() + amount);
        } else {
            ctx.config
                .set_animation_speed(ctx.config.animation_speed() - amount);
        }

        waterfall
            .pipeline()
            .set_speed(&ctx.gpu.queue, ctx.config.animation_speed());
        toast_manager.animation_speed_toast(ctx.config.animation_speed());
        return;
    }

    if let Some(ch @ ("_" | "-" | "+" | "=")) = event.character_released() {
        let amount = if ctx.window_state.modifiers_state.shift_key() {
            0.1
        } else {
            0.01
        };

        if matches!(ch, "-" | "_") {
            ctx.config
                .set_animation_offset(ctx.config.animation_offset() - amount);
        } else {
            ctx.config
                .set_animation_offset(ctx.config.animation_offset() + amount);
        }

        toast_manager.offset_toast(ctx.config.animation_offset());
    }
}
