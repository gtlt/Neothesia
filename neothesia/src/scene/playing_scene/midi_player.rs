use midi_file::midly::{MidiMessage, num::u4};

use crate::{
    output_manager::OutputConnection,
    song::{PlayerConfig, Song},
};
use neothesia_core::piano_layout;
use std::{collections::HashMap, time::Duration};

pub struct MidiPlayer {
    playback: midi_file::PlaybackState,
    output: OutputConnection,
    song: Song,
    play_along: PlayAlong,
    separate_channels: bool,
}

impl MidiPlayer {
    pub fn new(
        output: OutputConnection,
        song: Song,
        user_keyboard_range: piano_layout::KeyboardRange,
        separate_channels: bool,
    ) -> Self {
        Self::new_with_lead_in(
            output,
            song,
            user_keyboard_range,
            separate_channels,
            Duration::from_secs(3),
        )
    }

    pub fn new_with_lead_in(
        output: OutputConnection,
        song: Song,
        user_keyboard_range: piano_layout::KeyboardRange,
        separate_channels: bool,
        lead_in: Duration,
    ) -> Self {
        let targets = build_targets(&song, &user_keyboard_range, lead_in);
        let mut player = Self {
            playback: midi_file::PlaybackState::new(lead_in, song.file.tracks.clone()),
            output,
            play_along: PlayAlong::new(user_keyboard_range, targets),
            song,
            separate_channels,
        };
        // Let's reset programs,
        // for timestamp 0 most likely all programs will be 0, so this should clean any leftovers
        // from previous songs
        player.send_midi_programs_for_timestamp(&player.playback.time());
        player.update(Duration::ZERO);

        player
    }

    pub fn song(&self) -> &Song {
        &self.song
    }

    /// When playing: returns midi events
    ///
    /// When paused: returns None
    pub fn update(&mut self, delta: Duration) -> Vec<&midi_file::MidiEvent> {
        // Targets are stored on the playback clock, so file events are matched on that clock too
        let lead_in = *self.playback.leed_in();
        let events = self.playback.update(delta);

        events.iter().for_each(|event| {
            let config = &self.song.config.tracks[event.track_id];

            let channel = if self.separate_channels {
                event.track_color_id as u8
            } else {
                event.channel
            };
            match config.player {
                PlayerConfig::Auto => {
                    self.output // TODO: Send to multiple outputs
                        .midi_event(u4::new(channel), event.message);
                }
                // While listening, the app plays the hand tracks itself
                PlayerConfig::Human if self.play_along.is_listening() => {
                    self.output.midi_event(u4::new(channel), event.message);
                }
                PlayerConfig::Human => {
                    // Drums are never targets, see `build_targets`
                    if event.channel != DRUM_CHANNEL {
                        let time = event.timestamp + lead_in;
                        match event.message {
                            MidiMessage::NoteOn { key, .. } => {
                                self.play_along.file_note_on(key.as_int(), time);
                            }
                            MidiMessage::NoteOff { key, .. } => {
                                self.play_along.file_note_off(key.as_int());
                            }
                            _ => {}
                        }
                    }

                    // In Human mode note events from the file are targets for the player,
                    // not notes to be played by the synthesizer. Keep forwarding controller
                    // and other non-note events so the track still sounds as intended.
                    if should_forward_human_event(&event.message) {
                        self.output.midi_event(u4::new(channel), event.message);
                    }
                }
                PlayerConfig::Mute => {}
            }
        });

        events
    }

    fn clear(&mut self) {
        self.output.stop_all();
    }
}

impl Drop for MidiPlayer {
    fn drop(&mut self) {
        self.clear();
    }
}

impl MidiPlayer {
    pub fn pause_resume(&mut self) {
        if self.playback.is_paused() {
            self.resume();
        } else {
            self.pause();
        }
    }

    pub fn pause(&mut self) {
        self.clear();
        self.playback.pause();
    }

    pub fn resume(&mut self) {
        self.playback.resume();
        self.play_along.clear();
    }

    /// Back to time zero with a blank score, as if the player was just created.
    pub fn restart(&mut self) {
        self.clear();
        self.playback.set_time(Duration::ZERO);
        self.playback.resume();
        self.play_along.reset();
        self.send_midi_programs_for_timestamp(&Duration::ZERO);
        // Like in the constructor, events due at time zero (no lead-in) must not be skipped,
        // unlike in `set_time` which discards everything up to its target
        self.update(Duration::ZERO);
    }

    fn send_midi_programs_for_timestamp(&self, time: &Duration) {
        for (&channel, &p) in self.song.file.program_track.program_for_timestamp(time) {
            self.output.midi_event(
                u4::new(channel),
                midi_file::midly::MidiMessage::ProgramChange {
                    program: midi_file::midly::num::u7::new(p),
                },
            );
        }
    }

    pub fn set_time(&mut self, time: Duration) {
        self.playback.set_time(time);

        // Discard all of the events till that point
        let events = self.playback.update(Duration::ZERO);
        std::mem::drop(events);

        // Whatever the song was waiting for belongs to the place we just left
        self.play_along.clear();

        self.clear();
        self.send_midi_programs_for_timestamp(&time);
    }

    pub fn rewind(&mut self, delta: i64) {
        let mut time = self.playback.time();

        if delta < 0 {
            let delta = Duration::from_millis((-delta) as u64);
            time = time.saturating_sub(delta);
        } else {
            let delta = Duration::from_millis(delta as u64);
            time = time.saturating_add(delta);
        }

        self.set_time(time);
    }

    pub fn percentage_to_time(&self, p: f32) -> Duration {
        Duration::from_secs_f32((p * self.playback.length().as_secs_f32()).max(0.0))
    }

    pub fn time_to_percentage(&self, time: &Duration) -> f32 {
        time.as_secs_f32() / self.playback.length().as_secs_f32()
    }

    pub fn set_percentage_time(&mut self, p: f32) {
        self.set_time(self.percentage_to_time(p));
    }

    pub fn leed_in(&self) -> &Duration {
        self.playback.leed_in()
    }

    pub fn length(&self) -> Duration {
        self.playback.length()
    }

    pub fn percentage(&self) -> f32 {
        self.playback.percentage()
    }

    pub fn is_finished(&self) -> bool {
        self.playback.is_finished()
    }

    pub fn time(&self) -> Duration {
        self.playback.time()
    }

    pub fn time_without_lead_in(&self) -> f32 {
        self.playback.time().as_secs_f32() - self.playback.leed_in().as_secs_f32()
    }

    pub fn is_paused(&self) -> bool {
        self.playback.is_paused()
    }
}

impl MidiPlayer {
    pub fn play_along(&self) -> &PlayAlong {
        &self.play_along
    }

    /// Forwards the message to the output and, for a key press, tells if it was the right one.
    ///
    /// Returns `None` for anything that is not a key press.
    pub fn user_midi_event(&mut self, channel: u8, message: &MidiMessage) -> Option<PressVerdict> {
        self.output.midi_event(u4::new(channel), *message);

        match message {
            // NoteOn with velocity 0 is a release
            MidiMessage::NoteOn { key, vel } if vel.as_int() > 0 => {
                // A song that does not run is not being played along
                if self.playback.is_paused() || self.playback.is_finished() {
                    return Some(PressVerdict::Free);
                }

                Some(
                    self.play_along
                        .user_note_on(key.as_int(), self.playback.time()),
                )
            }
            _ => None,
        }
    }

    pub fn score(&self) -> Score {
        self.play_along.score()
    }

    /// True when at least one note has to be played by the user.
    pub fn has_targets(&self) -> bool {
        self.score().total > 0
    }

    /// While listening, the app plays the hand tracks and nothing is awaited.
    pub fn set_listening(&mut self, listening: bool) {
        if self.play_along.is_listening() == listening {
            return;
        }

        self.clear();
        self.play_along.set_listening(listening);
    }

    pub fn is_listening(&self) -> bool {
        self.play_along.is_listening()
    }
}

const DRUM_CHANNEL: u8 = 9;

/// How far ahead (on the playback clock) a press can be for a note to come.
const EARLY_WINDOW: Duration = Duration::from_millis(500);

/// Same, while the song waits for a note: only forgives chords not written exactly together.
const CHORD_WINDOW: Duration = Duration::from_millis(50);

/// What a key press of the player meant.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PressVerdict {
    /// Nothing to judge: no track is played by hand, the player is listening, the song
    /// does not run, or the key is the next one to play but its time has not come yet.
    Free,
    /// An awaited key, or one about to be awaited. Carries the colour id of its track.
    Right { track_color_id: usize },
    /// Not an awaited key.
    Wrong,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Score {
    /// Notes the player has to play in this song.
    pub total: usize,
    /// Notes whose latest attempt was right at the first press.
    pub first_try: usize,
    /// Notes whose latest attempt came after at least one wrong press.
    pub retried: usize,
    /// Wrong presses since the start or the last restart.
    pub wrong_presses: usize,
}

fn should_forward_human_event(message: &MidiMessage) -> bool {
    !matches!(
        message,
        MidiMessage::NoteOn { .. } | MidiMessage::NoteOff { .. }
    )
}

type NoteId = u8;

/// A note the player has to play.
#[derive(Debug, Clone, Copy)]
struct Target {
    /// Note-on time on the playback clock (lead-in included, so it is never negative)
    time: Duration,
    pitch: NoteId,
    track_color_id: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Outcome {
    Unplayed,
    FirstTry,
    Retried,
}

/// Notes of the tracks played by hand that fit on the user's keyboard, sorted by time and pitch.
fn build_targets(
    song: &Song,
    user_keyboard_range: &piano_layout::KeyboardRange,
    lead_in: Duration,
) -> Box<[Target]> {
    let mut targets: Vec<Target> = song
        .config
        .tracks
        .iter()
        .zip(song.file.tracks.iter())
        .filter(|(config, _)| config.player == PlayerConfig::Human)
        .flat_map(|(_, track)| track.notes.iter())
        .filter(|note| note.channel != DRUM_CHANNEL && user_keyboard_range.contains(note.note))
        .map(|note| Target {
            time: note.start + lead_in,
            pitch: note.note,
            track_color_id: note.track_color_id,
        })
        .collect();

    targets.sort_by_key(|t| (t.time, t.pitch));
    // Two notes of the same pitch at the same time can't be told apart in the file events,
    // and the second one could never be played
    targets.dedup_by_key(|t| (t.time, t.pitch));

    targets.into()
}

/// A target the song is waiting for
#[derive(Debug)]
struct AwaitedNote {
    target: usize,
    /// No wrong press since the song started waiting
    clean: bool,
}

#[derive(Debug)]
pub struct PlayAlong {
    user_keyboard_range: piano_layout::KeyboardRange,
    listening: bool,

    targets: Box<[Target]>,
    /// Latest attempt at each target, same indices as `targets`
    outcomes: Vec<Outcome>,
    score: Score,

    /// Notes required to proggres further in the song
    required_notes: HashMap<NoteId, AwaitedNote>,
    /// Keys pressed a little before their note, with the target they were pressed for.
    ///
    /// The press was judged against the playback clock, so it stays good for as long as
    /// that clock takes to reach the note, even when the song has to wait in between.
    early_presses: HashMap<NoteId, usize>,
    /// File notes that had NoteOn event, but no NoteOff yet, with their target
    in_proggres_file_notes: HashMap<NoteId, usize>,
}

impl PlayAlong {
    fn new(user_keyboard_range: piano_layout::KeyboardRange, targets: Box<[Target]>) -> Self {
        Self {
            user_keyboard_range,
            listening: false,
            outcomes: vec![Outcome::Unplayed; targets.len()],
            score: Score {
                total: targets.len(),
                ..Score::default()
            },
            targets,
            required_notes: Default::default(),
            early_presses: Default::default(),
            in_proggres_file_notes: Default::default(),
        }
    }

    fn set_outcome(&mut self, target: usize, outcome: Outcome) {
        fn counter(score: &mut Score, outcome: Outcome) -> Option<&mut usize> {
            match outcome {
                Outcome::Unplayed => None,
                Outcome::FirstTry => Some(&mut score.first_try),
                Outcome::Retried => Some(&mut score.retried),
            }
        }

        let previous = std::mem::replace(&mut self.outcomes[target], outcome);
        if let Some(n) = counter(&mut self.score, previous) {
            *n -= 1;
        }
        if let Some(n) = counter(&mut self.score, outcome) {
            *n += 1;
        }
    }

    fn find_target(&self, time: Duration, pitch: NoteId) -> Option<usize> {
        self.targets
            .binary_search_by_key(&(time, pitch), |t| (t.time, t.pitch))
            .ok()
    }

    /// A target of that pitch that starts after `clock`, but not later than `clock + window`
    fn upcoming_target(&self, pitch: NoteId, clock: Duration, window: Duration) -> Option<usize> {
        let first = self.targets.partition_point(|t| t.time <= clock);
        let limit = clock + window;

        self.targets[first..]
            .iter()
            .take_while(|t| t.time <= limit)
            .position(|t| t.pitch == pitch)
            .map(|i| first + i)
    }

    /// Is this the pitch of the next note to play, or of a note of the next chord?
    fn is_next_target(&self, pitch: NoteId, clock: Duration) -> bool {
        let first = self.targets.partition_point(|t| t.time <= clock);
        let Some(next) = self.targets.get(first) else {
            return false;
        };
        let limit = next.time + CHORD_WINDOW;

        self.targets[first..]
            .iter()
            .take_while(|t| t.time <= limit)
            .any(|t| t.pitch == pitch)
    }

    /// A note of a hand track reached by the playback, `time` being its playback clock time
    fn file_note_on(&mut self, note_id: NoteId, time: Duration) {
        if self.listening || !self.user_keyboard_range.contains(note_id) {
            return;
        }

        let Some(target) = self.find_target(time, note_id) else {
            return;
        };

        // Whatever note an early press was for, this one uses it up
        let early_press = self.early_presses.remove(&note_id);

        // Ignore overlapping notes
        if self.in_proggres_file_notes.contains_key(&note_id) {
            return;
        }

        if early_press == Some(target) {
            self.set_outcome(target, Outcome::FirstTry);
        } else {
            // Player never pressed that note, let it reach required_notes
            self.required_notes.insert(
                note_id,
                AwaitedNote {
                    target,
                    clean: true,
                },
            );
        }

        self.in_proggres_file_notes.insert(note_id, target);
    }

    fn file_note_off(&mut self, note_id: NoteId) {
        self.in_proggres_file_notes.remove(&note_id);
    }

    /// Judges a key press of the player, `clock` being the playback time.
    fn user_note_on(&mut self, note_id: NoteId, clock: Duration) -> PressVerdict {
        if self.listening || self.targets.is_empty() || !self.user_keyboard_range.contains(note_id)
        {
            return PressVerdict::Free;
        }

        // The song is waiting for this key
        if let Some(awaited) = self.required_notes.remove(&note_id) {
            let outcome = if awaited.clean {
                Outcome::FirstTry
            } else {
                Outcome::Retried
            };
            self.set_outcome(awaited.target, outcome);

            return PressVerdict::Right {
                track_color_id: self.targets[awaited.target].track_color_id,
            };
        }

        // Not there yet, but about to be.
        // Checked before the next case, so that a repeated note can be played early too.
        let window = if self.required_notes.is_empty() {
            EARLY_WINDOW
        } else {
            CHORD_WINDOW
        };
        if let Some(target) = self.upcoming_target(note_id, clock, window) {
            self.early_presses.insert(note_id, target);

            return PressVerdict::Right {
                track_color_id: self.targets[target].track_color_id,
            };
        }

        // Tapping again a note that is still sounding in the file and was already satisfied
        if let Some(&target) = self.in_proggres_file_notes.get(&note_id) {
            return PressVerdict::Right {
                track_color_id: self.targets[target].track_color_id,
            };
        }

        // The right key, only much too early: the song will wait for it anyway,
        // so this is no mistake. It does not count for the note either.
        if self.required_notes.is_empty() && self.is_next_target(note_id, clock) {
            return PressVerdict::Free;
        }

        self.score.wrong_presses += 1;
        for awaited in self.required_notes.values_mut() {
            awaited.clean = false;
        }

        PressVerdict::Wrong
    }

    pub fn clear(&mut self) {
        self.required_notes.clear();
        self.early_presses.clear();
        self.in_proggres_file_notes.clear();
    }

    /// Blank score, as for a song that was never played
    fn reset(&mut self) {
        self.clear();
        self.outcomes.fill(Outcome::Unplayed);
        self.score = Score {
            total: self.targets.len(),
            ..Score::default()
        };
    }

    fn set_listening(&mut self, listening: bool) {
        self.listening = listening;
        self.clear();
    }

    pub fn is_listening(&self) -> bool {
        self.listening
    }

    pub fn score(&self) -> Score {
        self.score
    }

    pub fn are_required_keys_pressed(&self) -> bool {
        self.required_notes.is_empty()
    }

    /// MIDI note numbers the song is waiting for
    pub fn awaited_notes(&self) -> impl Iterator<Item = u8> + '_ {
        self.required_notes.keys().copied()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use midi_file::midly::{
        Format, Header, Smf, Timing, TrackEvent, TrackEventKind,
        num::{u7, u15, u28},
    };

    /// Ticks of a 480 per quarter note file at 120 BPM
    const fn ms(x: u32) -> u32 {
        x * 24 / 25
    }

    #[derive(Clone, Copy)]
    struct Note {
        start: u32,
        dur: u32,
        key: u8,
    }

    const fn note(start: u32, dur: u32, key: u8) -> Note {
        Note { start, dur, key }
    }

    fn song(tracks: &[&[Note]], human: bool) -> Song {
        let tracks = tracks
            .iter()
            .map(|notes| {
                let mut events = Vec::new();
                for n in notes.iter() {
                    events.push((n.start, true, n.key));
                    events.push((n.start + n.dur, false, n.key));
                }
                // Note-offs first when on the same tick
                events.sort();

                let mut last = 0;
                events
                    .into_iter()
                    .map(|(tick, on, key)| {
                        let delta = tick - last;
                        last = tick;

                        let key = u7::new(key);
                        let message = if on {
                            MidiMessage::NoteOn {
                                key,
                                vel: u7::new(90),
                            }
                        } else {
                            MidiMessage::NoteOff {
                                key,
                                vel: u7::new(0),
                            }
                        };
                        TrackEvent {
                            delta: u28::new(delta),
                            kind: TrackEventKind::Midi {
                                channel: u4::new(0),
                                message,
                            },
                        }
                    })
                    .collect()
            })
            .collect();

        let smf = Smf {
            header: Header::new(Format::Parallel, Timing::Metrical(u15::new(480))),
            tracks,
        };
        let file = midi_file::MidiFile::from_smf("test", &smf).unwrap();

        let mut song = Song::new(file);
        if human {
            song.set_player(PlayerConfig::Human);
        }
        song
    }

    /// Plays the role of the scene: playback only advances while nothing is awaited
    struct Rig {
        player: MidiPlayer,
    }

    impl Rig {
        fn new(song: Song) -> Self {
            let player = MidiPlayer::new_with_lead_in(
                OutputConnection::DummyOutput,
                song,
                piano_layout::KeyboardRange::new(41..=72),
                false,
                Duration::ZERO,
            );
            Self { player }
        }

        fn human(tracks: &[&[Note]]) -> Self {
            Self::new(song(tracks, true))
        }

        fn advance(&mut self, millis: u64) {
            if self.player.play_along().are_required_keys_pressed() {
                let _ = self.player.update(Duration::from_millis(millis));
            }
        }

        fn press(&mut self, key: u8) -> Option<PressVerdict> {
            let message = MidiMessage::NoteOn {
                key: u7::new(key),
                vel: u7::new(100),
            };
            self.player.user_midi_event(0, &message)
        }

        fn awaited(&self) -> Vec<u8> {
            let mut keys: Vec<u8> = self.player.play_along().awaited_notes().collect();
            keys.sort();
            keys
        }

        fn blocked(&self) -> bool {
            !self.player.play_along().are_required_keys_pressed()
        }
    }

    fn right(track_color_id: usize) -> Option<PressVerdict> {
        Some(PressVerdict::Right { track_color_id })
    }

    fn score(total: usize, first_try: usize, retried: usize, wrong_presses: usize) -> Score {
        Score {
            total,
            first_try,
            retried,
            wrong_presses,
        }
    }

    #[test]
    fn awaited_note_pressed_is_right() {
        let mut rig = Rig::human(&[&[note(ms(1000), ms(500), 60)]]);

        rig.advance(900);
        assert!(!rig.blocked());
        assert_eq!(rig.player.score(), score(1, 0, 0, 0));

        rig.advance(110);
        assert_eq!(rig.awaited(), [60]);
        assert!(rig.blocked());

        assert_eq!(rig.press(60), right(0));
        assert!(!rig.blocked());
        assert_eq!(rig.player.score(), score(1, 1, 0, 0));
    }

    #[test]
    fn wrong_press_then_right_one_is_retried() {
        let mut rig = Rig::human(&[&[note(ms(1000), ms(500), 60)]]);
        rig.advance(1010);

        assert_eq!(rig.press(61), Some(PressVerdict::Wrong));
        assert!(rig.blocked());
        assert_eq!(rig.press(60), right(0));
        assert!(!rig.blocked());
        assert_eq!(rig.player.score(), score(1, 0, 1, 1));
    }

    #[test]
    fn early_press_is_right_and_first_try() {
        let mut rig = Rig::human(&[&[note(ms(1000), ms(500), 60)]]);
        rig.advance(700);

        assert_eq!(rig.press(60), right(0));
        assert_eq!(rig.player.score(), score(1, 0, 0, 0));

        rig.advance(310);
        assert!(rig.awaited().is_empty());
        assert!(!rig.blocked());
        assert_eq!(rig.player.score(), score(1, 1, 0, 0));
    }

    #[test]
    fn pressing_the_next_note_while_waiting_is_wrong() {
        let mut rig = Rig::human(&[&[note(ms(1000), ms(200), 60), note(ms(1400), ms(200), 62)]]);
        rig.advance(1010);
        assert_eq!(rig.awaited(), [60]);

        assert_eq!(rig.press(62), Some(PressVerdict::Wrong));
        assert_eq!(rig.press(60), right(0));
        assert_eq!(rig.player.score(), score(2, 0, 1, 1));
    }

    #[test]
    fn chord_needs_both_keys_in_any_order() {
        for order in [[60, 64], [64, 60]] {
            let mut rig =
                Rig::human(&[&[note(ms(1000), ms(500), 60), note(ms(1000), ms(500), 64)]]);
            rig.advance(1010);
            assert_eq!(rig.awaited(), [60, 64]);

            assert_eq!(rig.press(order[0]), right(0));
            assert!(rig.blocked());
            assert_eq!(rig.press(order[1]), right(0));
            assert!(!rig.blocked());
            assert_eq!(rig.player.score(), score(2, 2, 0, 0));
        }
    }

    #[test]
    fn loose_chord_second_note_is_right_while_first_is_awaited() {
        // 29 ticks are about 30 ms
        let mut rig = Rig::human(&[&[
            note(ms(1000), ms(500), 60),
            note(ms(1000) + 29, ms(500), 64),
        ]]);
        rig.advance(1010);
        assert_eq!(rig.awaited(), [60]);

        assert_eq!(rig.press(64), right(0));
        assert_eq!(rig.awaited(), [60]);
        assert_eq!(rig.press(60), right(0));
        assert!(!rig.blocked());

        // The file note arrives, the early press is consumed
        rig.advance(30);
        assert!(rig.awaited().is_empty());
        assert_eq!(rig.player.score(), score(2, 2, 0, 0));
    }

    #[test]
    fn tapping_a_satisfied_sounding_note_again_is_right() {
        let mut rig = Rig::human(&[&[note(ms(1000), ms(1000), 60)]]);
        rig.advance(1010);
        assert_eq!(rig.press(60), right(0));
        let before = rig.player.score();

        rig.advance(100);
        assert_eq!(rig.press(60), right(0));
        assert_eq!(rig.player.score(), before);
        assert!(rig.awaited().is_empty());
    }

    #[test]
    fn restart_resets_score_and_time() {
        let mut rig = Rig::human(&[&[note(ms(1000), ms(500), 60)]]);
        rig.advance(1010);
        rig.press(61);
        rig.press(60);
        assert_eq!(rig.player.score(), score(1, 0, 1, 1));

        rig.advance(100);
        rig.player.pause();
        assert!(rig.player.is_paused());

        rig.player.restart();
        assert_eq!(rig.player.score(), score(1, 0, 0, 0));
        assert_eq!(rig.player.time(), Duration::ZERO);
        assert!(!rig.player.is_paused());
        assert!(rig.awaited().is_empty());

        // And the song is playable again
        rig.advance(1010);
        assert_eq!(rig.awaited(), [60]);
    }

    #[test]
    fn replaying_a_retried_note_after_seeking_back_is_first_try() {
        let mut rig = Rig::human(&[&[note(ms(1000), ms(500), 60)]]);
        rig.advance(1010);
        rig.press(61);
        rig.press(60);
        assert_eq!(rig.player.score(), score(1, 0, 1, 1));

        rig.player.set_time(Duration::from_millis(500));
        rig.advance(510);
        assert_eq!(rig.awaited(), [60]);
        assert_eq!(rig.press(60), right(0));
        assert_eq!(rig.player.score(), score(1, 1, 0, 1));
    }

    #[test]
    fn seeking_forgets_what_the_song_waited_for() {
        let mut rig = Rig::human(&[&[note(ms(1000), ms(500), 60)]]);
        rig.advance(1010);
        assert!(rig.blocked());

        rig.player.set_time(Duration::ZERO);
        assert!(!rig.blocked());
        assert!(rig.awaited().is_empty());
    }

    #[test]
    fn targets_out_of_range_are_ignored() {
        let mut rig = Rig::human(&[&[
            note(ms(1000), ms(200), 60),
            note(ms(1500), ms(200), 90),
            note(ms(2000), ms(200), 30),
        ]]);
        assert_eq!(rig.player.score().total, 1);

        rig.advance(1010);
        assert_eq!(rig.press(60), right(0));

        rig.advance(1500);
        assert!(rig.awaited().is_empty());
        assert!(!rig.blocked());
        assert_eq!(rig.press(90), Some(PressVerdict::Free));
        assert_eq!(rig.player.score(), score(1, 1, 0, 0));
    }

    #[test]
    fn listening_awaits_nothing_and_judges_nothing() {
        let mut rig = Rig::human(&[&[note(ms(1000), ms(500), 60)]]);
        rig.player.set_listening(true);
        assert!(rig.player.is_listening());

        rig.advance(1010);
        assert!(rig.awaited().is_empty());
        assert!(!rig.blocked());
        assert_eq!(rig.press(60), Some(PressVerdict::Free));
        assert_eq!(rig.press(61), Some(PressVerdict::Free));
        assert_eq!(rig.player.score(), score(1, 0, 0, 0));

        rig.player.set_listening(false);
        assert!(!rig.player.is_listening());
        rig.player.set_time(Duration::from_millis(500));
        rig.advance(510);
        assert_eq!(rig.awaited(), [60]);
        assert_eq!(rig.press(60), right(0));
    }

    #[test]
    fn early_press_survives_a_long_wait_on_the_rest_of_its_chord() {
        // 29 ticks are about 30 ms
        let mut rig = Rig::human(&[&[
            note(ms(1000), ms(500), 60),
            note(ms(1000) + 29, ms(500), 64),
        ]]);
        rig.advance(1010);
        assert_eq!(rig.press(64), right(0));

        // The player takes a long time to find the other key of the chord
        rig.advance(5000);
        assert_eq!(rig.awaited(), [60]);
        assert_eq!(rig.press(60), right(0));

        rig.advance(30);
        assert!(rig.awaited().is_empty());
        assert_eq!(rig.player.score(), score(2, 2, 0, 0));
    }

    #[test]
    fn early_press_is_forgotten_by_a_pause() {
        let mut rig = Rig::human(&[&[note(ms(1000), ms(500), 60)]]);
        rig.advance(600);
        assert_eq!(rig.press(60), right(0));

        rig.player.pause();
        rig.player.resume();

        rig.advance(410);
        assert_eq!(rig.awaited(), [60]);
        assert_eq!(rig.player.score(), score(1, 0, 0, 0));
    }

    #[test]
    fn repeated_note_can_be_played_early_while_the_previous_one_sounds() {
        let mut rig = Rig::human(&[&[note(ms(1000), ms(450), 60), note(ms(1500), ms(450), 60)]]);
        rig.advance(1010);
        assert_eq!(rig.press(60), right(0));

        // The first note still sounds, the second one is 200 ms away
        rig.advance(290);
        assert_eq!(rig.press(60), right(0));

        rig.advance(300);
        assert!(rig.awaited().is_empty());
        assert_eq!(rig.player.score(), score(2, 2, 0, 0));
    }

    #[test]
    fn early_press_is_used_up_by_a_note_that_overlaps() {
        // The second note starts before the first one ends, the file never asks for it
        let mut rig = Rig::human(&[&[
            note(ms(1000), ms(600), 60),
            note(ms(1300), ms(600), 60),
            note(ms(3000), ms(200), 60),
        ]]);
        rig.advance(1010);
        assert_eq!(rig.press(60), right(0));
        rig.advance(100);
        assert_eq!(rig.press(60), right(0));

        // It must not be kept for the note that comes much later
        rig.advance(1900);
        assert_eq!(rig.awaited(), [60]);
    }

    #[test]
    fn next_note_pressed_much_too_early_is_no_mistake() {
        let mut rig = Rig::human(&[&[note(ms(2000), ms(200), 60), note(ms(2400), ms(200), 62)]]);
        rig.advance(500);

        // 1.5 s ahead: neither right nor wrong, and the song still asks for it
        assert_eq!(rig.press(60), Some(PressVerdict::Free));
        // The note after the next one is a mistake though
        assert_eq!(rig.press(62), Some(PressVerdict::Wrong));
        assert_eq!(rig.player.score(), score(2, 0, 0, 1));

        rig.advance(1510);
        assert_eq!(rig.awaited(), [60]);
        assert_eq!(rig.press(60), right(0));
        assert_eq!(rig.player.score(), score(2, 1, 0, 1));
    }

    #[test]
    fn presses_are_not_judged_while_paused() {
        let mut rig = Rig::human(&[&[note(ms(1000), ms(500), 60)]]);
        rig.advance(500);
        rig.player.pause();

        assert_eq!(rig.press(61), Some(PressVerdict::Free));
        assert_eq!(rig.press(60), Some(PressVerdict::Free));
        assert_eq!(rig.player.score(), score(1, 0, 0, 0));
    }

    #[test]
    fn auto_only_song_is_never_judged() {
        let mut rig = Rig::new(song(&[&[note(ms(1000), ms(500), 60)]], false));
        assert!(!rig.player.has_targets());

        rig.advance(1010);
        assert!(rig.awaited().is_empty());
        assert_eq!(rig.press(60), Some(PressVerdict::Free));
        assert_eq!(rig.press(61), Some(PressVerdict::Free));
        assert_eq!(rig.player.score(), Score::default());
    }

    #[test]
    fn only_key_presses_get_a_verdict() {
        let mut rig = Rig::human(&[&[note(ms(1000), ms(500), 60)]]);
        assert!(rig.player.has_targets());
        let release = MidiMessage::NoteOn {
            key: u7::new(60),
            vel: u7::new(0),
        };
        assert_eq!(rig.player.user_midi_event(0, &release), None);

        let off = MidiMessage::NoteOff {
            key: u7::new(60),
            vel: u7::new(0),
        };
        assert_eq!(rig.player.user_midi_event(0, &off), None);
        assert_eq!(rig.player.score(), score(1, 0, 0, 0));
    }

    #[test]
    fn track_colour_follows_the_track() {
        let mut rig = Rig::human(&[
            &[note(ms(1000), ms(200), 60)],
            &[note(ms(1500), ms(200), 62)],
        ]);
        rig.advance(1010);
        assert_eq!(rig.press(60), right(0));
        rig.advance(500);
        assert_eq!(rig.press(62), right(1));
    }
}
