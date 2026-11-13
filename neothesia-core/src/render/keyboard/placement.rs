use piano_layout::{KeyboardLayout, KeyboardRange, Sizing};

use crate::utils::{Point, Size};

/// Share of the window height the keyboard takes, as long as its keys are slim enough.
const HEIGHT_SHARE: f32 = 0.2;
/// The keyboard never takes more of the window height than this.
const MAX_HEIGHT_SHARE: f32 = 0.34;
/// A white key is never drawn squatter than this, height over width.
///
/// A real one is about 6.4. With all 88 keys in a 16:9 window, [`HEIGHT_SHARE`] gives 5.8.
const MIN_KEY_ASPECT: f32 = 4.5;

/// Size of the keys and place of the keyboard in a window.
#[derive(Debug, Clone)]
pub struct KeyboardPlacement {
    pub layout: KeyboardLayout,
    /// Top left corner of the keyboard
    pub pos: Point<f32>,
}

impl KeyboardPlacement {
    /// Lays the keyboard out at the bottom of a window.
    ///
    /// A short keyboard stretched over a wide window would get keys wider than they are
    /// tall. To keep them looking like piano keys the keyboard first grows taller, then
    /// stops filling the width and stays in the middle.
    pub fn new(window: Size<f32>, range: KeyboardRange) -> Self {
        let white_count = range.white_count().max(1) as f32;

        let mut neutral_width = window.width / white_count;
        let mut height = window.height * HEIGHT_SHARE;

        if height < neutral_width * MIN_KEY_ASPECT {
            height = (neutral_width * MIN_KEY_ASPECT).min(window.height * MAX_HEIGHT_SHARE);
            neutral_width = neutral_width.min(height / MIN_KEY_ASPECT);
        }

        let layout = KeyboardLayout::from_range(Sizing::new(neutral_width, height), range);

        let pos = Point::new(
            ((window.width - layout.width) / 2.0).max(0.0),
            window.height - layout.height,
        );

        Self { layout, pos }
    }

    /// Does the keyboard leave room on its sides?
    pub fn has_margins(&self) -> bool {
        self.pos.x >= 1.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn placement(
        width: f32,
        height: f32,
        range: std::ops::RangeInclusive<u8>,
    ) -> KeyboardPlacement {
        KeyboardPlacement::new(Size::new(width, height), KeyboardRange::new(range))
    }

    fn key_aspect(placement: &KeyboardPlacement) -> f32 {
        placement.layout.sizing.neutral_height / placement.layout.sizing.neutral_width
    }

    #[test]
    fn full_keyboard_fills_the_width_at_a_fifth_of_the_height() {
        let p = placement(1920.0, 1080.0, 21..=108);

        assert_eq!(p.layout.height, 216.0);
        assert_eq!(p.layout.width, 1920.0);
        assert_eq!(p.pos, Point::new(0.0, 864.0));
        assert!(!p.has_margins());
    }

    #[test]
    fn mid_sized_keyboard_only_grows_taller() {
        // 61 keys
        let p = placement(1920.0, 1080.0, 36..=96);

        assert_eq!(p.layout.width, 1920.0);
        assert!(p.layout.height > 216.0);
        assert!(p.layout.height <= 1080.0 * MAX_HEIGHT_SHARE);
        assert!((key_aspect(&p) - MIN_KEY_ASPECT).abs() < 0.001);
    }

    #[test]
    fn short_keyboard_in_a_wide_window_is_centered() {
        // 32 keys on an ultrawide screen
        let p = placement(3440.0, 1440.0, 41..=72);

        assert!((key_aspect(&p) - MIN_KEY_ASPECT).abs() < 0.001);
        assert!((p.layout.height - 1440.0 * MAX_HEIGHT_SHARE).abs() < 0.001);
        assert!(p.layout.width < 3440.0);
        assert!(p.has_margins());

        let right_margin = 3440.0 - p.pos.x - p.layout.width;
        assert!((p.pos.x - right_margin).abs() < 0.001);
        assert!((p.pos.y + p.layout.height - 1440.0).abs() < 0.001);
    }

    #[test]
    fn keys_never_get_squat() {
        for (width, height) in [
            (670.0, 620.0),
            (1080.0, 720.0),
            (1920.0, 1080.0),
            (5120.0, 1440.0),
        ] {
            for range in [21..=108, 36..=96, 48..=72, 41..=72, 60..=72] {
                let p = placement(width, height, range.clone());
                assert!(
                    key_aspect(&p) >= MIN_KEY_ASPECT - 0.001,
                    "{width}x{height} {range:?}: {}",
                    key_aspect(&p)
                );
                assert!(p.layout.width <= width + 0.001);
                assert!(p.layout.height <= height * MAX_HEIGHT_SHARE + 0.001);
            }
        }
    }
}
