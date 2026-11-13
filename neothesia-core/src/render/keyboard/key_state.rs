use std::time::Duration;

use crate::config::ColorSchemaV1;
use wgpu_jumpstart::Color;

/// Seconds a key takes to go most of the way down, or back up
const TRAVEL_TIME: f32 = 0.03;

/// An awaited key starts to pulse after this many seconds,
/// to catch the eye of a player who can't find it
const PULSE_DELAY: f32 = 2.0;
/// Seconds
const PULSE_PERIOD: f32 = 1.3;

#[derive(Debug, Clone)]
pub struct KeyState {
    is_sharp: bool,

    pressed_by_file: Option<Color>,
    /// The note of the file is for the player: the key lights up, but only the player moves it
    expected: bool,
    /// The song does not go on until the player presses the key
    awaited: bool,
    awaited_for: f32,

    pressed_by_user: Option<Color>,

    /// How far down the key is: 0 at rest, 1 at the bottom
    travel: f32,
}

impl KeyState {
    pub fn new(is_sharp: bool) -> Self {
        Self {
            is_sharp,

            pressed_by_file: None,
            expected: false,
            awaited: false,
            awaited_for: 0.0,

            pressed_by_user: None,

            travel: 0.0,
        }
    }

    pub fn is_sharp(&self) -> bool {
        self.is_sharp
    }

    pub fn pressed_by_user(&self) -> Option<&Color> {
        self.pressed_by_user.as_ref()
    }

    pub fn pressed_by_file(&self) -> Option<&Color> {
        self.pressed_by_file.as_ref()
    }

    /// Color of this key in a color scheme
    pub fn scheme_color(&self, scheme: &ColorSchemaV1) -> Color {
        let (r, g, b) = if self.is_sharp {
            scheme.dark
        } else {
            scheme.base
        };

        Color::from_rgba8(r, g, b, 1.0)
    }

    pub fn set_pressed_by_user(&mut self, is: bool, scheme: &ColorSchemaV1) {
        self.pressed_by_user = is.then(|| self.scheme_color(scheme));
    }

    pub fn pressed_by_file_on(&mut self, scheme: &ColorSchemaV1) {
        self.pressed_by_file = Some(self.scheme_color(scheme));
        self.expected = false;
    }

    /// The file reached a note the player has to play: the key lights up but stays up
    pub fn expected_by_file_on(&mut self, scheme: &ColorSchemaV1) {
        self.pressed_by_file = Some(self.scheme_color(scheme));
        self.expected = true;
    }

    pub fn pressed_by_file_off(&mut self) {
        self.pressed_by_file = None;
        self.expected = false;
        self.awaited = false;
    }

    /// Tells the key if the song is waiting for the player to press it.
    ///
    /// Returns true when that changed.
    pub fn set_awaited(&mut self, awaited: bool) -> bool {
        if self.awaited == awaited {
            return false;
        }

        self.awaited = awaited;
        self.awaited_for = 0.0;
        true
    }

    /// Is the song waiting for the player on this key?
    pub fn is_awaited(&self) -> bool {
        self.awaited && self.pressed_by_user.is_none()
    }

    fn is_down(&self) -> bool {
        self.pressed_by_user.is_some() || (self.pressed_by_file.is_some() && !self.expected)
    }

    /// How far down the key is: 0 at rest, 1 at the bottom
    pub fn travel(&self) -> f32 {
        self.travel
    }

    /// Color the key takes, and how much of it
    pub fn tint(&self) -> Option<(Color, f32)> {
        if let Some(color) = self.pressed_by_user {
            Some((color, 1.0))
        } else if let Some(color) = self.pressed_by_file {
            let strength = if self.awaited {
                awaited_strength(self.awaited_for)
            } else if self.expected {
                EXPECTED_STRENGTH
            } else {
                1.0
            };
            Some((color, strength))
        } else {
            None
        }
    }

    /// Moves the key towards where it should be.
    ///
    /// Returns true as long as the key has to be drawn again.
    pub fn animate(&mut self, delta: Duration) -> bool {
        let delta = delta.as_secs_f32();
        let mut changed = false;

        let target = if self.is_down() { 1.0 } else { 0.0 };
        if self.travel != target {
            let step = 1.0 - (-delta / TRAVEL_TIME).exp();
            self.travel += (target - self.travel) * step;
            if (self.travel - target).abs() < 0.01 {
                self.travel = target;
            }
            changed = true;
        }

        if self.is_awaited() {
            self.awaited_for += delta;
            changed |= self.awaited_for >= PULSE_DELAY;
        }

        changed
    }
}

/// How much of its color a key takes when its note is for the player.
/// A bit less than a key that is down, the difference tells them apart.
const EXPECTED_STRENGTH: f32 = 0.9;

/// A steady light at first, then a slow pulse
fn awaited_strength(awaited_for: f32) -> f32 {
    const DIP: f32 = 0.38;

    let pulsing_for = awaited_for - PULSE_DELAY;
    if pulsing_for <= 0.0 {
        return EXPECTED_STRENGTH;
    }

    let phase = pulsing_for / PULSE_PERIOD * std::f32::consts::TAU;
    EXPECTED_STRENGTH - DIP * (0.5 - 0.5 * phase.cos())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scheme() -> ColorSchemaV1 {
        ColorSchemaV1 {
            base: (200, 100, 50),
            dark: (100, 50, 25),
        }
    }

    fn settle(key: &mut KeyState) {
        for _ in 0..100 {
            key.animate(Duration::from_millis(16));
        }
    }

    #[test]
    fn key_played_by_the_file_goes_down_and_back() {
        let mut key = KeyState::new(false);
        assert_eq!(key.travel(), 0.0);
        assert!(!key.animate(Duration::from_millis(16)));

        key.pressed_by_file_on(&scheme());
        assert!(key.animate(Duration::from_millis(16)));
        assert!(key.travel() > 0.0 && key.travel() < 1.0);
        settle(&mut key);
        assert_eq!(key.travel(), 1.0);
        assert!(!key.animate(Duration::from_millis(16)));

        key.pressed_by_file_off();
        settle(&mut key);
        assert_eq!(key.travel(), 0.0);
        assert!(key.tint().is_none());
    }

    #[test]
    fn expected_key_is_lit_but_stays_up_until_the_player_presses_it() {
        let mut key = KeyState::new(false);

        key.expected_by_file_on(&scheme());
        assert!(key.set_awaited(true));
        assert!(!key.set_awaited(true));
        settle(&mut key);
        assert!(key.is_awaited());
        assert_eq!(key.travel(), 0.0);
        assert!(key.tint().is_some());

        key.set_pressed_by_user(true, &scheme());
        key.set_awaited(false);
        assert!(!key.is_awaited());
        settle(&mut key);
        assert_eq!(key.travel(), 1.0);
        assert_eq!(key.tint().unwrap().1, 1.0);

        // Released before the end of the note: still lit, up, and no longer pulsing
        key.set_pressed_by_user(false, &scheme());
        settle(&mut key);
        assert_eq!(key.travel(), 0.0);
        assert_eq!(key.tint().unwrap().1, EXPECTED_STRENGTH);
        assert!(!key.animate(Duration::from_secs(10)));

        key.pressed_by_file_off();
        assert!(key.tint().is_none());
    }

    #[test]
    fn awaited_key_pulses_only_after_a_while() {
        let mut key = KeyState::new(true);
        key.expected_by_file_on(&scheme());
        key.set_awaited(true);

        let steady = key.tint().unwrap().1;
        assert!(!key.animate(Duration::from_millis(500)));
        assert_eq!(key.tint().unwrap().1, steady);

        // Half a period into the pulse the light is at its lowest
        assert!(key.animate(Duration::from_secs_f32(
            PULSE_DELAY - 0.5 + PULSE_PERIOD / 2.0
        )));
        assert!(key.tint().unwrap().1 < steady - 0.3);

        // And a full period brings it back
        key.animate(Duration::from_secs_f32(PULSE_PERIOD / 2.0));
        assert!((key.tint().unwrap().1 - steady).abs() < 0.01);
    }

    #[test]
    fn black_keys_take_the_dark_color() {
        let mut white = KeyState::new(false);
        let mut black = KeyState::new(true);
        white.pressed_by_file_on(&scheme());
        black.pressed_by_file_on(&scheme());

        assert!(white.pressed_by_file().unwrap().r > black.pressed_by_file().unwrap().r);
    }
}
