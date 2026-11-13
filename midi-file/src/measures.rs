use midly::{MetaMessage, TrackEvent, TrackEventKind};
use std::collections::HashMap;

/// A time signature change: `(pulse, numerator, denominator as a power of two)`.
pub type TimeSignatureChange = (u64, u8, u8);

/// Collects the time signature events of all tracks, sorted by pulse.
pub fn time_signature_changes(track_events: &[Vec<TrackEvent>]) -> Vec<TimeSignatureChange> {
    // Same pulse in several tracks is one event, like for tempo events.
    let mut changes: HashMap<u64, TimeSignatureChange> = HashMap::new();

    for events in track_events.iter() {
        let mut pulses: u64 = 0;
        for event in events.iter() {
            pulses += event.delta.as_int() as u64;

            if let TrackEventKind::Meta(MetaMessage::TimeSignature(num, den_pow, _, _)) =
                &event.kind
            {
                changes.insert(pulses, (pulses, *num, *den_pow));
            }
        }
    }

    let mut changes: Vec<_> = changes.into_values().collect();
    changes.sort_by_key(|c| c.0);
    changes
}

/// Length of a bar in pulses, `None` for a signature that cannot give a usable bar.
fn bar_len(pulses_per_quarter_note: u16, numerator: u8, denominator_pow: u8) -> Option<u64> {
    let denominator = 1u64.checked_shl(denominator_pow as u32)?;
    let len = pulses_per_quarter_note as u64 * 4 * numerator as u64 / denominator;
    (len > 0).then_some(len)
}

/// Endless iterator over bar start pulses, the first bar starts at pulse 0.
///
/// `changes` must be sorted by pulse. The signature is 4/4 until the first change.
/// A change on a bar line applies from that bar, a change inside a bar cuts it short
/// and starts a new bar at the position of the change.
pub fn bar_starts(
    changes: &[TimeSignatureChange],
    pulses_per_quarter_note: u16,
) -> impl Iterator<Item = u64> + '_ {
    let mut pos = 0u64;
    let mut next_change = 0usize;
    let mut signature = (4u8, 2u8);

    std::iter::from_fn(move || {
        while let Some(&(pulse, num, den_pow)) = changes.get(next_change) {
            if pulse > pos {
                break;
            }
            signature = (num, den_pow);
            next_change += 1;
        }

        let start = pos;

        // Bad signatures fall back to 4/4, and a bar is never empty,
        // so the iterator always moves forward.
        let len = bar_len(pulses_per_quarter_note, signature.0, signature.1)
            .or_else(|| bar_len(pulses_per_quarter_note, 4, 2))
            .unwrap_or(1);
        let end = start + len;

        pos = match changes.get(next_change) {
            Some(&(pulse, _, _)) if pulse < end => pulse,
            _ => end,
        };

        Some(start)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const PPQ: u16 = 480;

    fn first(changes: &[TimeSignatureChange], n: usize) -> Vec<u64> {
        bar_starts(changes, PPQ).take(n).collect()
    }

    fn old_formula(n: usize) -> Vec<u64> {
        (0..n as u64).map(|id| id * PPQ as u64 * 4).collect()
    }

    #[test]
    fn no_signature_is_four_four() {
        assert_eq!(first(&[], 6), old_formula(6));
    }

    #[test]
    fn explicit_four_four_matches_old_formula() {
        assert_eq!(first(&[(0, 4, 2)], 6), old_formula(6));
    }

    #[test]
    fn three_four() {
        assert_eq!(first(&[(0, 3, 2)], 4), vec![0, 1440, 2880, 4320]);
    }

    #[test]
    fn two_four() {
        assert_eq!(first(&[(0, 2, 2)], 4), vec![0, 960, 1920, 2880]);
    }

    #[test]
    fn six_eight_is_three_quarter_notes() {
        assert_eq!(first(&[(0, 6, 3)], 4), vec![0, 1440, 2880, 4320]);
    }

    #[test]
    fn two_two() {
        assert_eq!(first(&[(0, 2, 1)], 3), vec![0, 1920, 3840]);
    }

    #[test]
    fn seven_sixteen() {
        // 7 sixteenth notes = 7 * 120 pulses
        assert_eq!(first(&[(0, 7, 4)], 4), vec![0, 840, 1680, 2520]);
    }

    #[test]
    fn change_on_bar_line_applies_from_that_bar() {
        // 4/4 for two bars, then 3/4
        let changes = [(0, 4, 2), (3840, 3, 2)];
        assert_eq!(first(&changes, 5), vec![0, 1920, 3840, 5280, 6720]);
    }

    #[test]
    fn change_in_the_middle_of_a_bar_starts_a_new_bar() {
        // 4/4 bar cut at 1000, then 3/4 bars from there
        let changes = [(1000, 3, 2)];
        assert_eq!(first(&changes, 4), vec![0, 1000, 2440, 3880]);
    }

    #[test]
    fn zero_numerator_does_not_loop_forever() {
        let changes = [(0, 0, 2)];
        assert_eq!(first(&changes, 4), old_formula(4));
    }

    #[test]
    fn absurd_denominator_does_not_loop_forever() {
        assert_eq!(first(&[(0, 4, 200)], 3), old_formula(3));
        assert_eq!(first(&[(0, 1, 30)], 3), old_formula(3));
    }

    #[test]
    fn zero_pulses_per_quarter_note_still_advances() {
        let v: Vec<u64> = bar_starts(&[], 0).take(3).collect();
        assert_eq!(v, vec![0, 1, 2]);
    }
}
