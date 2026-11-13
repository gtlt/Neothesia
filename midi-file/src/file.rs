use crate::{
    MidiTrack,
    measures::{bar_starts, time_signature_changes},
    program_track::ProgramTrack,
    tempo_track::TempoTrack,
};
use midly::{Format, Smf, Timing};
use std::{fs, path::Path, sync::Arc};

#[derive(Debug, Clone)]
pub struct MidiFile {
    pub name: String,
    pub format: Format,
    pub tracks: Arc<[MidiTrack]>,
    pub program_track: ProgramTrack,
    pub tempo_track: TempoTrack,
    pub measures: Arc<[std::time::Duration]>,
}

impl MidiFile {
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self, String> {
        let name = path
            .as_ref()
            .file_name()
            .ok_or(String::from("File not found"))?
            .to_string_lossy()
            .to_string();

        let data = match fs::read(path) {
            Ok(buff) => buff,
            Err(_) => return Err(String::from("Could Not Open File")),
        };

        let smf = match Smf::parse(&data) {
            Ok(smf) => smf,
            Err(_) => return Err(String::from("Midi Parsing Error (midly lib)")),
        };

        Self::from_parsed_smf(name, &smf)
    }

    pub fn from_smf(name: impl Into<String>, smf: &Smf<'_>) -> Result<Self, String> {
        Self::from_parsed_smf(name.into(), smf)
    }

    fn from_parsed_smf(name: String, smf: &Smf<'_>) -> Result<Self, String> {
        let u_per_quarter_note: u16 = match smf.header.timing {
            Timing::Metrical(t) => t.as_int(),
            Timing::Timecode(_fps, _u) => {
                return Err(String::from("Midi With Timecode Timing, Not Supported!"));
            }
        };

        if smf.tracks.is_empty() {
            return Err(String::from("Midi File Has No Tracks"));
        }

        let tempo_track = TempoTrack::build(&smf.tracks, u_per_quarter_note);

        let mut track_color_id = 0;
        let tracks: Vec<MidiTrack> = smf
            .tracks
            .iter()
            .enumerate()
            .map(|(id, events)| {
                let track = MidiTrack::new(id, track_color_id, &tempo_track, events);

                if !track.notes.is_empty() {
                    track_color_id += 1;
                }

                track
            })
            .collect();

        let measures = {
            let last_note_end = tracks
                .iter()
                .fold(std::time::Duration::ZERO, |last, track| {
                    if let Some(note) = track.notes.last() {
                        last.max(note.start + note.duration)
                    } else {
                        last
                    }
                });

            let signatures = time_signature_changes(&smf.tracks);
            let mut bar_pulses = bar_starts(&signatures, u_per_quarter_note);

            let mut measures = Vec::new();
            let mut time = std::time::Duration::ZERO;
            while time <= last_note_end {
                let Some(pulses) = bar_pulses.next() else {
                    break;
                };
                time = tempo_track.pulses_to_duration(pulses);
                measures.push(time);
            }

            measures
        };

        let program_track = ProgramTrack::new(&tracks);

        Ok(Self {
            name,
            format: smf.header.format,
            tracks: tracks.into(),
            program_track,
            tempo_track,
            measures: measures.into(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use midly::{
        Header, MetaMessage, MidiMessage, TrackEvent, TrackEventKind,
        num::{u4, u7, u24, u28},
    };
    use std::time::Duration;

    fn event(delta: u32, kind: TrackEventKind<'static>) -> TrackEvent<'static> {
        TrackEvent {
            delta: u28::new(delta),
            kind,
        }
    }

    fn note(delta: u32, on: bool) -> TrackEvent<'static> {
        let message = if on {
            MidiMessage::NoteOn {
                key: u7::new(60),
                vel: u7::new(90),
            }
        } else {
            MidiMessage::NoteOff {
                key: u7::new(60),
                vel: u7::new(0),
            }
        };
        event(
            delta,
            TrackEventKind::Midi {
                channel: u4::new(0),
                message,
            },
        )
    }

    #[test]
    fn measures_follow_six_eight_signature() {
        // 6/8 at 480 pulses per quarter note and 120 BPM: a bar is 3 quarter notes, 1.5 s.
        let conductor = vec![
            event(
                0,
                TrackEventKind::Meta(MetaMessage::TimeSignature(6, 3, 24, 8)),
            ),
            event(
                0,
                TrackEventKind::Meta(MetaMessage::Tempo(u24::new(500_000))),
            ),
            event(0, TrackEventKind::Meta(MetaMessage::EndOfTrack)),
        ];
        // Notes lasting 8 quarter notes, so 4 s.
        let notes = vec![
            note(0, true),
            note(480 * 8, false),
            event(0, TrackEventKind::Meta(MetaMessage::EndOfTrack)),
        ];

        let smf = Smf {
            header: Header::new(Format::Parallel, Timing::Metrical(480.into())),
            tracks: vec![conductor, notes],
        };

        let file = MidiFile::from_smf("six-eight", &smf).unwrap();

        // The last bar is the first one past the end of the last note.
        let expected: Vec<Duration> = [0.0, 1.5, 3.0, 4.5]
            .into_iter()
            .map(Duration::from_secs_f64)
            .collect();
        assert_eq!(&*file.measures, expected.as_slice());
    }
}
