use std::collections::VecDeque;

use crate::{
    NeothesiaEvent,
    context::Context,
    output_manager::OutputDescriptor,
    scene::menu_scene::settings::{KeyboardSize, RangeDetection},
    song::Song,
};

type InputDescriptor = midi_io::MidiInputPort;

pub struct UiState {
    pub outputs: Vec<OutputDescriptor>,
    pub selected_output: Option<OutputDescriptor>,

    pub inputs: Vec<InputDescriptor>,
    pub selected_input: Option<InputDescriptor>,

    pub is_loading: bool,

    pub song: Option<Song>,

    pub range_detection: RangeDetection,
    pub keyboard_size: KeyboardSize,

    page_stack: VecDeque<Page>,
}

impl UiState {
    pub fn new(ctx: &Context, song: Option<Song>) -> Self {
        let mut page_stack = VecDeque::new();
        page_stack.push_front(Page::Main);

        Self {
            outputs: Vec::new(),
            selected_output: None,
            inputs: Vec::new(),
            selected_input: None,
            is_loading: false,
            song,

            page_stack,
            range_detection: RangeDetection::default(),
            keyboard_size: KeyboardSize::from_range(ctx.config.piano_range()),
        }
    }

    pub fn song(&self) -> Option<&Song> {
        self.song.as_ref()
    }

    pub fn is_loading(&self) -> bool {
        self.is_loading
    }

    pub fn current(&self) -> &Page {
        self.page_stack.front().unwrap()
    }

    pub fn go_to(&mut self, page: Page) {
        self.page_stack.push_front(page);
    }

    pub fn go_back(&mut self) {
        self.range_detection.stop_detection();
        match self.page_stack.len() {
            1 => {
                // Last page in the stack, let's go to exit page
                self.page_stack.push_front(Page::Exit);
            }
            _ => {
                self.page_stack.pop_front();
            }
        }
    }
}

impl UiState {
    pub fn tick(&mut self, ctx: &mut Context) {
        self.outputs = ctx.output_manager.outputs();
        self.inputs = ctx.input_manager.inputs();

        if self.selected_output.is_none() {
            if let Some(name) = ctx.config.output() {
                if let Some(out) = self
                    .outputs
                    .iter()
                    .find(|output| output.to_string().as_str() == name)
                {
                    self.selected_output = Some(out.clone());
                } else {
                    self.selected_output = self.outputs.first().cloned();
                }
            } else {
                self.selected_output = Some(OutputDescriptor::DummyOutput);
            }
        }

        if self.selected_input.is_none() {
            let saved = ctx.config.input();

            let exact = self
                .inputs
                .iter()
                .find(|input| Some(input.to_string().as_str()) == saved);

            // Same device, plugged in a different order than last time.
            let renumbered = || {
                let saved = without_port_numbers(saved?);
                self.inputs
                    .iter()
                    .find(|input| without_port_numbers(&input.to_string()) == saved)
            };

            self.selected_input = exact.or_else(renumbered).or(self.inputs.first()).cloned();
        }
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum Page {
    Exit,
    Main,
    Settings,
    TrackSelection,
}

pub fn connect_io(data: &UiState, ctx: &mut Context) {
    if let Some(out) = data.selected_output.clone() {
        let out = match out {
            #[cfg(feature = "synth")]
            OutputDescriptor::Synth(_) => OutputDescriptor::Synth(ctx.config.soundfont_path()),
            o => o,
        };

        ctx.output_manager.connect(out);
        ctx.output_manager
            .connection()
            .set_gain(ctx.config.audio_gain());
    }

    if let Some(port) = data.selected_input.clone() {
        ctx.input_manager.connect_input(port);
    }
}

pub fn play(data: &UiState, ctx: &mut Context) {
    let Some(song) = data.song.as_ref() else {
        return;
    };

    connect_io(data, ctx);

    ctx.proxy
        .send_event(NeothesiaEvent::Play(song.clone()))
        .ok();
}

pub fn freeplay(data: &UiState, ctx: &mut Context) {
    connect_io(data, ctx);

    ctx.proxy
        .send_event(NeothesiaEvent::FreePlay(data.song.clone()))
        .ok();
}

/// Port name without the client and port numbers that ALSA appends to it,
/// e.g. `"Keyboard:Keyboard MIDI 1 24:0"` gives `"Keyboard:Keyboard MIDI 1"`.
///
/// Those numbers depend on the order devices were plugged in, so they can't
/// be relied on to find a device again.
fn without_port_numbers(name: &str) -> &str {
    let Some((device, numbers)) = name.rsplit_once(' ') else {
        return name;
    };
    let Some((client, port)) = numbers.split_once(':') else {
        return name;
    };

    let is_number = |text: &str| !text.is_empty() && text.bytes().all(|b| b.is_ascii_digit());
    if is_number(client) && is_number(port) {
        device
    } else {
        name
    }
}

#[cfg(test)]
mod tests {
    use super::without_port_numbers;

    #[test]
    fn alsa_numbers_are_dropped() {
        assert_eq!(
            without_port_numbers("Keyboard:Keyboard MIDI 1 24:0"),
            "Keyboard:Keyboard MIDI 1"
        );
        assert_eq!(
            without_port_numbers("Keyboard:Keyboard MIDI 1 20:0"),
            without_port_numbers("Keyboard:Keyboard MIDI 1 24:0")
        );
    }

    #[test]
    fn other_names_are_left_alone() {
        for name in ["Keyboard", "Keyboard MIDI 1", "Port 1:2a", "Mix 12:", ""] {
            assert_eq!(without_port_numbers(name), name);
        }
    }
}
