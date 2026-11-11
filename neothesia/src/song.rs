use midi_file::MidiTrack;

use crate::context::Context;

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum PlayerConfig {
    Mute,
    Auto,
    Human,
}

impl PlayerConfig {
    /// Mode given to the tracks of a newly opened song.
    pub fn default_for(ctx: &Context) -> Self {
        if ctx.config.play_along_by_default() {
            Self::Human
        } else {
            Self::Auto
        }
    }
}

#[derive(Debug, Clone)]
pub struct TrackConfig {
    pub track_id: usize,
    pub player: PlayerConfig,
    pub visible: bool,
}

#[derive(Default, Debug, Clone)]
pub struct SongConfig {
    pub tracks: Box<[TrackConfig]>,
}

/// A track with nothing but drums can't be played on a keyboard.
fn is_drums(track: &MidiTrack) -> bool {
    track.has_drums && !track.has_other_than_drums
}

impl SongConfig {
    fn new(tracks: &[MidiTrack], player: PlayerConfig) -> Self {
        let tracks: Vec<_> = tracks
            .iter()
            .map(|t| {
                let is_drums = is_drums(t);
                TrackConfig {
                    track_id: t.track_id,
                    player: if is_drums { PlayerConfig::Auto } else { player },
                    visible: !is_drums,
                }
            })
            .collect();
        Self {
            tracks: tracks.into(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Song {
    pub file: midi_file::MidiFile,
    pub config: SongConfig,
}

impl Song {
    /// Every track is played by the app.
    pub fn new(file: midi_file::MidiFile) -> Self {
        let config = SongConfig::new(&file.tracks, PlayerConfig::Auto);
        Self { file, config }
    }

    /// Tracks start in the mode the user asked for in the settings.
    pub fn with_user_defaults(file: midi_file::MidiFile, ctx: &Context) -> Self {
        let config = SongConfig::new(&file.tracks, PlayerConfig::default_for(ctx));
        Self { file, config }
    }

    /// Changes the mode of every track that can be played on a keyboard.
    pub fn set_player(&mut self, player: PlayerConfig) {
        for (config, track) in self.config.tracks.iter_mut().zip(self.file.tracks.iter()) {
            if !is_drums(track) {
                config.player = player;
            }
        }
    }

    pub fn from_env(ctx: &Context) -> Option<Self> {
        let args: Vec<String> = std::env::args().collect();
        let midi_file = if args.len() > 1 {
            midi_file::MidiFile::new(&args[1]).ok()
        } else if let Some(last) = ctx.config.last_opened_song() {
            midi_file::MidiFile::new(last).ok()
        } else {
            None
        };

        Some(Self::with_user_defaults(midi_file?, ctx))
    }
}
