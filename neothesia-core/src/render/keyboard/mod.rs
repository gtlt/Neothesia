use std::time::Duration;

use crate::{
    config::NoteNaming,
    render::TextRenderer,
    utils::{Point, Size},
};

use piano_layout::range::KeyboardRange;
use wgpu_jumpstart::{Gpu, TransformUniform, Uniform};

mod key_state;
mod pipeline;
mod placement;

pub use key_state::KeyState;
pub use placement::KeyboardPlacement;

use pipeline::{KeyInstance, KeyboardPipeline, kind};

struct Label {
    buffer: glyphon::Buffer,
    left: f32,
    top: f32,
    bounds: glyphon::TextBounds,
    color: glyphon::Color,
}

pub struct KeyboardRenderer {
    placement: KeyboardPlacement,
    window: Size<f32>,

    key_states: Vec<KeyState>,
    note_naming: NoteNaming,

    pipeline: KeyboardPipeline,
    /// Keys have to be sent to the GPU again
    keys_changed: bool,

    text_renderer: TextRenderer,
    labels: Vec<Label>,
    labels_changed: bool,
}

impl KeyboardRenderer {
    /// `window` is the size of what the keyboard is drawn in, in logical pixels.
    pub fn new(
        gpu: &Gpu,
        transform_uniform: &Uniform<TransformUniform>,
        text_renderer: TextRenderer,
        range: KeyboardRange,
        window: Size<f32>,
    ) -> Self {
        let key_states: Vec<KeyState> = range
            .iter()
            .map(|id| KeyState::new(id.is_black()))
            .collect();

        // Every key, a shadow for the black ones, and what is around
        let size_hint = range.count() + range.black_count() + 4;

        Self {
            placement: KeyboardPlacement::new(window, range),
            window,

            key_states,
            note_naming: NoteNaming::default(),

            pipeline: KeyboardPipeline::new(gpu, transform_uniform, size_hint),
            keys_changed: true,

            text_renderer,
            labels: Vec::new(),
            labels_changed: true,
        }
    }

    pub fn reset_notes(&mut self) {
        for key in self.key_states.iter_mut() {
            key.pressed_by_file_off();
        }
        self.invalidate_keys();
    }

    pub fn range(&self) -> &KeyboardRange {
        &self.placement.layout.range
    }

    pub fn key_states(&self) -> &[KeyState] {
        &self.key_states
    }

    /// Call [`Self::invalidate_keys`] after changing a key
    pub fn key_states_mut(&mut self) -> &mut [KeyState] {
        &mut self.key_states
    }

    pub fn placement(&self) -> &KeyboardPlacement {
        &self.placement
    }

    pub fn pos(&self) -> &Point<f32> {
        &self.placement.pos
    }

    pub fn layout(&self) -> &piano_layout::KeyboardLayout {
        &self.placement.layout
    }

    /// The window changed size
    pub fn resize(&mut self, window: Size<f32>) {
        self.placement = KeyboardPlacement::new(window, self.range().clone());
        self.window = window;
        self.invalidate_keys();
        self.labels_changed = true;
    }

    pub fn set_note_naming(&mut self, note_naming: NoteNaming) {
        self.note_naming = note_naming;
        self.labels_changed = true;
    }

    /// Keys changed and have to be drawn again
    pub fn invalidate_keys(&mut self) {
        self.keys_changed = true;
    }

    #[profiling::function]
    fn rebuild_keys(&mut self) {
        let instances = self.pipeline.instances();
        instances.clear();

        let layout = &self.placement.layout;
        let origin = self.placement.pos;
        let felt = felt_height(layout.height);

        // What is left of the window on both sides of the keyboard
        if self.placement.has_margins() {
            let right_x = origin.x + layout.width;
            for (x, width, side) in [
                (0.0, origin.x, 0.0),
                (right_x, self.window.width - right_x, 1.0),
            ] {
                instances.push(KeyInstance {
                    position: [x, origin.y - felt],
                    size: [width, layout.height + felt],
                    tint: [0.0; 4],
                    params: [kind::CHEEK, side, 0.0, 0.0],
                });
            }
        }

        instances.push(KeyInstance {
            position: origin.into(),
            size: [layout.width, layout.height],
            tint: [0.0; 4],
            params: [kind::BED, 0.0, 0.0, 0.0],
        });

        let key_instance = |key: &piano_layout::Key, state: &KeyState| {
            let tint = match state.tint() {
                Some((color, strength)) => {
                    let [r, g, b, _] = color.into_linear_rgba();
                    [r, g, b, strength]
                }
                None => [0.0; 4],
            };

            let kind = if key.kind().is_sharp() {
                kind::BLACK_KEY
            } else {
                kind::WHITE_KEY
            };

            KeyInstance {
                position: [origin.x + key.x(), origin.y],
                size: [key.width(), key.height()],
                tint,
                params: [kind, state.travel(), 0.0, 0.0],
            }
        };

        let neutral = layout.keys.iter().filter(|key| key.kind().is_neutral());
        let sharp = layout.keys.iter().filter(|key| key.kind().is_sharp());

        for key in neutral {
            instances.push(key_instance(key, &self.key_states[key.id()]));
        }

        for key in sharp.clone() {
            // The shadow spreads around the key
            let margin = key.width() * 0.4;

            instances.push(KeyInstance {
                position: [origin.x + key.x() - margin, origin.y],
                size: [key.width() + margin * 2.0, key.height() + margin],
                tint: [0.0; 4],
                params: [
                    kind::BLACK_KEY_SHADOW,
                    self.key_states[key.id()].travel(),
                    key.width(),
                    key.height(),
                ],
            });
        }

        for key in sharp {
            instances.push(key_instance(key, &self.key_states[key.id()]));
        }

        instances.push(KeyInstance {
            position: [origin.x, origin.y - felt],
            size: [layout.width, felt],
            tint: [0.0; 4],
            params: [kind::FELT, 0.0, 0.0, 0.0],
        });

        self.pipeline.prepare();
    }

    #[profiling::function]
    fn rebuild_labels(&mut self) {
        self.labels.clear();

        let font_system = crate::font_system::font_system();
        let font_system = &mut font_system.borrow_mut();

        let layout = &self.placement.layout;
        let origin = self.placement.pos;

        let range_start = layout.range.start() as usize;
        for key in layout
            .keys
            .iter()
            .filter(|key| self.note_naming != NoteNaming::Octaves || key.note_id() == 0)
        {
            let name = self.note_naming.note_name(key.note_id());
            if name.is_empty() {
                continue;
            }

            let label = if self.note_naming == NoteNaming::Octaves {
                let oct_number = (key.id() + range_start) / 12;
                format!("{name}{}", oct_number as i8 - 1)
            } else {
                name.to_string()
            };

            let x = origin.x + key.x();
            let y = origin.y;

            let w = key.width();
            let h = key.height();

            let is_sharp = key.kind().is_sharp();

            // Three letters have to fit in the width of a key, with room to spare
            let size = if is_sharp {
                (w * 0.42).max(7.0)
            } else {
                (w * 0.36).min(h * 0.11).max(8.0)
            };

            // Just above the front face of the key, see the shader for its height
            let front = if is_sharp {
                (h * 0.09).clamp(4.0, w * 0.8)
            } else {
                (h * 0.035).clamp(3.0, w * 0.25)
            };
            let top = y + h - front - size * 1.5;

            let mut buffer = glyphon::Buffer::new(font_system, glyphon::Metrics::new(size, size));
            buffer.set_size(Some(w), Some(h));
            buffer.set_wrap(glyphon::Wrap::None);
            buffer.set_text(
                &label,
                &glyphon::Attrs::new().family(glyphon::Family::SansSerif),
                glyphon::Shaping::Basic,
                Some(glyphon::cosmic_text::Align::Center),
            );
            buffer.shape_until_scroll(font_system, false);

            self.labels.push(Label {
                buffer,
                left: x,
                top,
                bounds: glyphon::TextBounds {
                    left: x.round() as i32,
                    top: y.round() as i32,
                    right: x.round() as i32 + w.round() as i32,
                    bottom: y.round() as i32 + h.round() as i32,
                },
                color: if self.note_naming == NoteNaming::Octaves {
                    glyphon::Color::rgba(0, 0, 0, 150)
                } else if is_sharp {
                    glyphon::Color::rgb(235, 235, 235)
                } else {
                    glyphon::Color::rgb(52, 50, 48)
                },
            });
        }
    }

    /// `delta` moves the keys that are going down or up.
    #[profiling::function]
    pub fn update(&mut self, delta: Duration, physical_size: dpi::PhysicalSize<u32>, scale: f32) {
        let mut moving = false;
        for key in self.key_states.iter_mut() {
            moving |= key.animate(delta);
        }

        if self.keys_changed || moving {
            self.rebuild_keys();
            self.keys_changed = false;
        }

        if self.labels_changed {
            self.rebuild_labels();
            self.labels_changed = false;
        }

        // Has to be done every frame, even when nothing changed: the glyph atlas is shared,
        // and it drops the glyphs nobody asked for since its last trim
        self.text_renderer.update_from_iter(
            physical_size,
            scale,
            self.labels.iter().map(|label| glyphon::TextArea {
                buffer: &label.buffer,
                left: label.left,
                top: label.top,
                scale: 1.0,
                bounds: label.bounds,
                default_color: label.color,
                custom_glyphs: &[],
            }),
        );
    }

    #[profiling::function]
    pub fn render<'rpass>(&'rpass self, render_pass: &mut wgpu_jumpstart::RenderPass<'rpass>) {
        self.pipeline.render(render_pass);
        self.text_renderer.render(render_pass);
    }
}

/// Height of the strip of felt at the back of the keys
fn felt_height(keyboard_height: f32) -> f32 {
    (keyboard_height * 0.016).round().max(3.0)
}
