use midi_file::midly::{self, MidiMessage, live::LiveEvent};
use winit::event_loop::EventLoopProxy;

use crate::NeothesiaEvent;

pub struct InputManager {
    input: midi_io::MidiInputManager,
    tx: EventLoopProxy<NeothesiaEvent>,
    current_connection: Option<(midi_io::MidiInputPort, midi_io::MidiInputConnection)>,
}

impl InputManager {
    pub fn new(tx: EventLoopProxy<NeothesiaEvent>) -> Self {
        let input = midi_io::MidiInputManager::new().unwrap();
        Self {
            input,
            tx,
            current_connection: None,
        }
    }

    pub fn inputs(&self) -> Vec<midi_io::MidiInputPort> {
        self.input.inputs()
    }

    pub fn connect_input(&mut self, port: midi_io::MidiInputPort) {
        let tx = self.tx.clone();

        if let Some((current, _)) = self.current_connection.as_ref()
            && current == &port
        {
            return;
        }

        // Close the connection first, as Windows does not like it when we hold 2 connections
        self.current_connection = None;

        self.current_connection = midi_io::MidiInputManager::connect_input(port, move |message| {
            let event = LiveEvent::parse(message).unwrap();

            if let LiveEvent::Midi { channel, message } = event {
                match message {
                    // Some keyboards send NoteOn event with vel 0 instead of NoteOff
                    midly::MidiMessage::NoteOn { key, vel } if vel == 0 => {
                        tx.send_event(NeothesiaEvent::MidiInput {
                            channel: channel.as_int(),
                            message: MidiMessage::NoteOff { key, vel },
                        })
                        .ok();
                    }
                    message => {
                        tx.send_event(NeothesiaEvent::MidiInput {
                            channel: channel.as_int(),
                            message,
                        })
                        .ok();
                    }
                }
            }
        });
    }
}

/// Squeezes a key velocity into `min..=127`: a soft press stays audible, and a
/// harder one is still louder. `min` of 0 leaves the velocity as it is.
pub fn lift_velocity(velocity: u8, min: u8) -> u8 {
    if velocity == 0 || min == 0 {
        return velocity;
    }

    let velocity = u32::from(velocity.min(127));
    let min = u32::from(min.min(127));
    // Rounded to the nearest, so that the hardest press stays at 127.
    (min + (velocity * (127 - min) + 63) / 127) as u8
}

/// Applies [`lift_velocity`] to key presses, other messages are left alone.
pub fn with_min_velocity(message: MidiMessage, min: u8) -> MidiMessage {
    match message {
        MidiMessage::NoteOn { key, vel } => MidiMessage::NoteOn {
            key,
            vel: midly::num::u7::new(lift_velocity(vel.as_int(), min)),
        },
        message => message,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use midly::num::u7;

    #[test]
    fn min_velocity_of_zero_changes_nothing() {
        for velocity in 0..=127 {
            assert_eq!(lift_velocity(velocity, 0), velocity);
        }
    }

    #[test]
    fn lifted_velocity_stays_between_min_and_max() {
        for min in 1..=127 {
            assert_eq!(lift_velocity(127, min), 127);
            for velocity in 1..=127 {
                let lifted = lift_velocity(velocity, min);
                assert!((min..=127).contains(&lifted), "{velocity} with {min}");
            }
        }
    }

    #[test]
    fn harder_press_is_never_softer() {
        for min in 0..=127 {
            for velocity in 1..127 {
                assert!(lift_velocity(velocity, min) <= lift_velocity(velocity + 1, min));
            }
        }
    }

    #[test]
    fn release_is_not_turned_into_a_press() {
        assert_eq!(lift_velocity(0, 60), 0);
    }

    #[test]
    fn only_key_presses_are_changed() {
        let key = u7::new(60);

        let press = MidiMessage::NoteOn {
            key,
            vel: u7::new(1),
        };
        assert_eq!(
            with_min_velocity(press, 60),
            MidiMessage::NoteOn {
                key,
                vel: u7::new(61)
            }
        );

        let release = MidiMessage::NoteOff {
            key,
            vel: u7::new(1),
        };
        assert_eq!(with_min_velocity(release, 60), release);
    }
}
