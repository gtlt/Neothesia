use crate::{config::NoteNaming, utils::Point};

use super::{KeyboardRenderer, TextRenderer, waterfall::NoteList};

#[derive(Default)]
struct LabelsCache {
    labels: Option<[(glyphon::Buffer, f32); 12]>,
    neutral_width: f32,
    naming: NoteNaming,
}

impl LabelsCache {
    #[profiling::function]
    fn get(&mut self, keyboard: &KeyboardRenderer) -> &[(glyphon::Buffer, f32); 12] {
        let font_system = crate::font_system::font_system();
        let font_system = &mut font_system.borrow_mut();

        let neutral_width = keyboard.layout().sizing.neutral_width;

        if self.labels.is_none() || self.neutral_width != neutral_width {
            let label_width = label_size(keyboard.layout());

            let labels = std::array::from_fn(|note_id| {
                let mut buffer = glyphon::Buffer::new(
                    font_system,
                    glyphon::Metrics::new(label_width, label_width),
                );
                buffer.set_wrap(glyphon::Wrap::None);
                buffer.set_text(
                    self.naming.note_name(note_id as u8),
                    &glyphon::Attrs::new().family(glyphon::Family::SansSerif),
                    glyphon::Shaping::Basic,
                    None,
                );
                buffer.shape_until_scroll(font_system, false);

                let (text_width, _) = TextRenderer::measure(&buffer);
                (buffer, text_width)
            });

            self.labels = Some(labels);
            self.neutral_width = neutral_width;
        }

        self.labels.as_ref().unwrap()
    }
}

/// Font size of the labels: the widest name has to fit in the narrowest note,
/// but has to stay readable on a full keyboard in a small window
fn label_size(layout: &piano_layout::KeyboardLayout) -> f32 {
    let sharp_width = layout.sizing.sharp_width;
    (sharp_width * 0.55).clamp(14.0_f32.min(sharp_width), sharp_width)
}

pub struct NoteLabels {
    pos: Point<f32>,
    notes: NoteList,
    labels_cache: LabelsCache,
    text_renderer: TextRenderer,
}

impl NoteLabels {
    pub fn new(
        pos: Point<f32>,
        notes: &NoteList,
        naming: NoteNaming,
        text_renderer: TextRenderer,
    ) -> Self {
        Self {
            pos,
            notes: notes.clone(),
            labels_cache: LabelsCache {
                naming,
                ..Default::default()
            },
            text_renderer,
        }
    }

    pub fn set_pos(&mut self, pos: Point<f32>) {
        self.pos = pos;
    }

    #[profiling::function]
    pub fn update(
        &mut self,
        physical_size: dpi::PhysicalSize<u32>,
        scale: f32,
        keyboard: &KeyboardRenderer,
        animation_speed: f32,
        time: f32,
    ) {
        let layout = keyboard.layout();
        let range_start = layout.range.start() as usize;
        // The label sits at the bottom of its note, with a bit of room under it
        let label_height = label_size(layout) * 1.5;

        let labels = self.labels_cache.get(keyboard);
        let animation_speed = animation_speed / scale;

        let label_y = |note: &midi_file::MidiNote| {
            self.pos.y - (note.start.as_secs_f32() - time) * animation_speed - label_height
        };

        // Notes are sorted by start: the ones whose label already went under the top of
        // the keyboard are all at the front, no need to walk through them
        let first = self
            .notes
            .inner
            .partition_point(|note| label_y(note) > keyboard.pos().y);

        let iter = self.notes.inner[first..]
            .iter()
            .filter(|note| layout.range.contains(note.note) && note.channel != 9)
            .map(|note| {
                let (buffer, text_width) = &labels[(note.note % 12) as usize];

                let key = &layout.keys[note.note as usize - range_start];
                let x = self.pos.x + key.x() + (key.width() - text_width) / 2.0;

                (buffer, x, label_y(note))
            })
            // Stop iteration once we reach top of the screen
            .take_while(|(_buffer, _x, y)| *y > 0.0)
            .map(|(buffer, left, top)| glyphon::TextArea {
                buffer,
                left,
                top,
                scale: 1.0,
                bounds: glyphon::TextBounds {
                    left: 0,
                    top: 0,
                    right: i32::MAX,
                    bottom: i32::MAX,
                },
                default_color: glyphon::Color::rgb(255, 255, 255),
                custom_glyphs: &[],
            });

        self.text_renderer
            .update_from_iter(physical_size, scale, iter);
    }

    pub fn render<'rpass>(&'rpass mut self, render_pass: &mut wgpu_jumpstart::RenderPass<'rpass>) {
        self.text_renderer.render(render_pass);
    }
}
