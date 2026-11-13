use std::time::Duration;

use midi_file::midly::MidiMessage;
use neothesia_core::{
    config::ColorSchemaV1,
    piano_layout,
    render::{KeyboardKeyState, KeyboardPlacement},
    utils::{Point, Size},
};
use piano_layout::KeyboardRange;

use crate::{
    config::Config,
    context::Context,
    render::KeyboardRenderer,
    song::{PlayerConfig, SongConfig},
};

use super::midi_player::PressVerdict;

/// A key that is not the one the song asked for
const WRONG_KEY_COLORS: ColorSchemaV1 = ColorSchemaV1 {
    base: (222, 92, 86),
    dark: (160, 52, 48),
};

pub struct Keyboard {
    renderer: KeyboardRenderer,
    song_config: SongConfig,
    pressed_by_user_colors: ColorSchemaV1,
    /// The app plays the tracks of the player too
    listening: bool,
}

fn window_size(ctx: &Context) -> Size<f32> {
    Size::new(
        ctx.window_state.logical_size.width,
        ctx.window_state.logical_size.height,
    )
}

impl Keyboard {
    pub fn new(ctx: &Context, song_config: SongConfig) -> Self {
        let mut renderer = KeyboardRenderer::new(
            &ctx.gpu,
            &ctx.transform,
            ctx.text_renderer_factory.new_renderer(),
            piano_layout::KeyboardRange::new(ctx.config.piano_range()),
            window_size(ctx),
        );
        renderer.set_note_naming(ctx.config.keyboard_note_naming());

        let v = (255.0 * 0.3) as u8;
        let dark = (v, v, v);

        let v = (255.0 * 0.5) as u8;
        let base = (v, v, v);

        Self {
            renderer,
            song_config,
            pressed_by_user_colors: ColorSchemaV1 { base, dark },
            listening: false,
        }
    }

    pub fn set_song_config(&mut self, song_config: SongConfig) {
        self.song_config = song_config;
    }

    pub fn set_pressed_by_user_colors(&mut self, colors: ColorSchemaV1) {
        self.pressed_by_user_colors = colors;
    }

    /// While listening the app plays every track, the keys of the player go down by themselves
    pub fn set_listening(&mut self, listening: bool) {
        self.listening = listening;
    }

    pub fn renderer(&self) -> &KeyboardRenderer {
        &self.renderer
    }

    pub fn key_states(&self) -> &[KeyboardKeyState] {
        self.renderer.key_states()
    }

    pub fn placement(&self) -> &KeyboardPlacement {
        self.renderer.placement()
    }

    pub fn layout(&self) -> &piano_layout::KeyboardLayout {
        self.renderer.layout()
    }

    pub fn range(&self) -> &KeyboardRange {
        &self.layout().range
    }

    pub fn pos(&self) -> &Point<f32> {
        self.renderer.pos()
    }

    #[profiling::function]
    pub fn resize(&mut self, ctx: &Context) {
        self.renderer.resize(window_size(ctx));
    }

    pub fn update(&mut self, ctx: &Context, delta: Duration) {
        self.renderer.update(
            delta,
            ctx.window_state.physical_size,
            ctx.window_state.scale_factor as f32,
        )
    }

    pub fn render<'pass>(&'pass self, rpass: &mut wgpu_jumpstart::RenderPass<'pass>) {
        self.renderer.render(rpass)
    }

    pub fn reset_notes(&mut self) {
        self.renderer.reset_notes()
    }

    pub fn user_midi_event(&mut self, message: &MidiMessage) {
        let colors = self.pressed_by_user_colors.clone();
        self.user_key(message, &colors);
    }

    /// Same as [`Self::user_midi_event`], for a key press that was judged against the song:
    /// the key takes the color of its track when it is the right one.
    pub fn judged_user_midi_event(
        &mut self,
        config: &Config,
        message: &MidiMessage,
        verdict: Option<PressVerdict>,
    ) {
        let colors = match verdict {
            Some(PressVerdict::Right { track_color_id }) => {
                config.color_schema()[track_color_id % config.color_schema().len()].clone()
            }
            Some(PressVerdict::Wrong) => WRONG_KEY_COLORS,
            Some(PressVerdict::Free) | None => self.pressed_by_user_colors.clone(),
        };
        self.user_key(message, &colors);
    }

    fn user_key(&mut self, message: &MidiMessage, colors: &ColorSchemaV1) {
        let range_start = self.range().start() as usize;

        let (is_on, key) = match message {
            MidiMessage::NoteOn { key, vel } => (vel.as_int() > 0, key.as_int()),
            MidiMessage::NoteOff { key, .. } => (false, key.as_int()),
            _ => return,
        };

        if self.range().contains(key) {
            let id = key as usize - range_start;
            let key = &mut self.renderer.key_states_mut()[id];

            key.set_pressed_by_user(is_on, colors);
            self.renderer.invalidate_keys();
        }
    }

    /// Tells the keys which ones the song is waiting for
    pub fn set_awaited_notes(&mut self, notes: impl Iterator<Item = u8>) {
        // One bit per MIDI note
        let awaited = notes.fold(0u128, |set, note| set | 1 << (note & 127));
        let range_start = self.range().start();

        let mut changed = false;
        for (id, key) in self.renderer.key_states_mut().iter_mut().enumerate() {
            let note = range_start as usize + id;
            changed |= key.set_awaited(awaited >> note & 1 == 1);
        }

        if changed {
            self.renderer.invalidate_keys();
        }
    }

    pub fn file_midi_events(&mut self, config: &Config, events: &[&midi_file::MidiEvent]) {
        let range_start = self.range().start() as usize;

        for e in events {
            let track = &self.song_config.tracks[e.track_id];
            if !track.visible {
                continue;
            }

            let (is_on, key) = match e.message {
                MidiMessage::NoteOn { key, .. } => (true, key.as_int()),
                MidiMessage::NoteOff { key, .. } => (false, key.as_int()),
                _ => continue,
            };

            if self.range().contains(key) && e.channel != 9 {
                let id = key as usize - range_start;
                let key = &mut self.renderer.key_states_mut()[id];

                if is_on {
                    let color =
                        &config.color_schema()[e.track_color_id % config.color_schema().len()];

                    if track.player == PlayerConfig::Human && !self.listening {
                        key.expected_by_file_on(color);
                    } else {
                        key.pressed_by_file_on(color);
                    }
                } else {
                    key.pressed_by_file_off();
                }

                self.renderer.invalidate_keys();
            }
        }
    }
}
