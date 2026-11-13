use std::{sync::Arc, time::Duration};

use crate::{
    render::{QuadInstance, QuadRenderer},
    utils::Point,
};

pub struct GuidelineRenderer {
    pos: Point<f32>,

    layout: piano_layout::KeyboardLayout,
    vertical_guidelines: bool,
    horizontal_guidelines: bool,
    measure_line_height: f32,

    cache: Vec<QuadInstance>,
    /// The cache can be empty and valid: there may be nothing to draw
    cache_is_valid: bool,
    measures: Arc<[Duration]>,
}

impl GuidelineRenderer {
    pub fn new(
        layout: piano_layout::KeyboardLayout,
        pos: Point<f32>,
        vertical_guidelines: bool,
        horizontal_guidelines: bool,
        measures: Arc<[Duration]>,
    ) -> Self {
        Self {
            pos,
            layout,
            vertical_guidelines,
            horizontal_guidelines,
            measure_line_height: 1.0,
            cache: Vec::new(),
            cache_is_valid: false,
            measures,
        }
    }

    pub fn set_pos(&mut self, pos: Point<f32>) {
        self.pos = pos;
        self.invalidate_cache();
    }

    pub fn set_layout(&mut self, layout: piano_layout::KeyboardLayout) {
        self.layout = layout;
        self.invalidate_cache();
    }

    fn invalidate_cache(&mut self) {
        self.cache.clear();
        self.cache_is_valid = false;
    }

    pub fn set_measure_line_height(&mut self, height: f32) {
        self.measure_line_height = height;
    }

    /// Reupload instances to GPU
    fn reupload(&mut self) {
        // A keyboard that does not fill the window gets its lanes framed
        if self.pos.x >= 1.0 {
            for x in [self.pos.x - 1.0, self.pos.x + self.layout.width] {
                self.cache.push(QuadInstance {
                    position: [x, 0.0],
                    size: [1.0, self.pos.y],
                    color: [0.1, 0.1, 0.1, 1.0],
                    ..Default::default()
                });
            }
        }

        if !self.vertical_guidelines {
            return;
        }

        for key in self
            .layout
            .keys
            .iter()
            .filter(|key| key.note_id() == 0 || key.note_id() == 5)
        {
            let x = self.pos.x + key.x();
            let y = 0.0;

            let w = 1.0;
            // Down to the keyboard
            let h = self.pos.y;

            let color = if key.note_id() == 0 {
                [0.2, 0.2, 0.2, 1.0]
            } else {
                [0.05, 0.05, 0.05, 1.0]
            };

            self.cache.push(QuadInstance {
                position: [x, y],
                size: [w, h],
                color,
                border_radius: [0.0, 0.0, 0.0, 0.0],
                ..Default::default()
            });
        }
    }

    #[profiling::function]
    fn update_horizontal_guidelines(
        &mut self,
        quads: &mut QuadRenderer,
        animation_speed: f32,
        scale: f32,
        time: f32,
    ) {
        // Without snapping to whole physical pixels, the line seems to flicker a little bit
        let snap = |v: f32| (v * scale).round() / scale;

        // The ticker the line, the dimer it has to be to still look decent
        let intensity = 0.05 / self.measure_line_height.max(1.0);

        // Measures are sorted: no need to walk through the ones already played
        let first = self
            .measures
            .partition_point(|bar| bar.as_secs_f32() < time);

        for masure in &self.measures[first..] {
            // As wide as the keyboard
            let x = self.pos.x;
            let y = snap(self.pos.y - (masure.as_secs_f32() - time) * animation_speed);

            let w = self.layout.width;
            let h = snap(self.measure_line_height).max(1.0 / scale);

            if y < 0.0 {
                break;
            }

            quads.layer().push(QuadInstance {
                position: [x, y],
                size: [w, h],
                color: [intensity, intensity, intensity, 1.0],
                border_radius: [0.0, 0.0, 0.0, 0.0],
                ..Default::default()
            });
        }
    }

    #[profiling::function]
    pub fn update(
        &mut self,
        quads: &mut QuadRenderer,
        animation_speed: f32,
        scale: f32,
        time: f32,
    ) {
        if !self.cache_is_valid {
            self.reupload();
            self.cache_is_valid = true;
        }

        if self.horizontal_guidelines {
            let animation_speed = animation_speed / scale;
            self.update_horizontal_guidelines(quads, animation_speed, scale, time);
        }

        quads.layer().extend(&self.cache);
    }
}
