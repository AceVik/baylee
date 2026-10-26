//! An eleven-minute instrumental remembrance, from three complete CC0 editions.
//! The source tempo maps, pitches, durations and dynamics are prepared off-line;
//! playback reads bounded records directly, without parsing or allocating.
use super::{Movement, arrangement::Instrument};
use crate::music::{
    RATE, SampleSet,
    orchestra::{Orchestra, Touch},
};

const DATA: &[u8] = include_bytes!("memorial/performance.bin");
const HEADER: usize = 12;
const RECORD: usize = 20;

pub(super) const fn contains(movement: Movement) -> bool {
    matches!(movement, Movement::Title | Movement::Lobby)
}

#[derive(Clone, Copy)]
pub(in crate::music) struct Event {
    frame: u32,
    length: u32,
    gain: f32,
    pan: f32,
    pub instrument: Instrument,
    pub pitch: u8,
}
fn number(at: usize) -> u32 {
    u32::from_le_bytes(
        DATA[at..at + 4]
            .try_into()
            .expect("checked-in performance record"),
    )
}
fn count() -> usize {
    number(4) as usize
}
fn end() -> u32 {
    number(8)
}
fn event(index: usize) -> Event {
    let at = HEADER + index * RECORD;
    Event {
        frame: number(at),
        length: number(at + 4),
        gain: f32::from_bits(number(at + 8)),
        pan: f32::from_bits(number(at + 12)),
        instrument: match DATA[at + 16] {
            0 => Instrument::Harp,
            3 => Instrument::Violin,
            5 => Instrument::Cello,
            11 => Instrument::Piano,
            13 => Instrument::Flute,
            _ => unreachable!("validated memorial instrumentation"),
        },
        pitch: DATA[at + 17],
    }
}
/// All prepared events, also used to warm the recorded-instrument pitch cache.
pub(in crate::music) fn events() -> impl Iterator<Item = Event> {
    (0..count()).map(event)
}

#[derive(Default)]
pub(super) struct Performance {
    frame: u32,
    cursor: usize,
}
impl Performance {
    /// Start precisely due notes. Bound the next observation to 50 ms so scene
    /// changes still respond during a long rest. The reverberation never resets.
    pub(super) fn play(&mut self, orchestra: &mut Orchestra, samples: SampleSet) -> u32 {
        while self.cursor < count() {
            let note = event(self.cursor);
            if note.frame > self.frame {
                break;
            }
            let touch = Touch::at(note.pan).attack(0.035).release(0.32);
            orchestra.note_with_samples(
                note.instrument,
                note.pitch,
                note.length as f32 / RATE as f32,
                note.gain,
                touch,
                samples,
            );
            self.cursor += 1;
        }
        let next = if self.cursor < count() {
            event(self.cursor).frame
        } else {
            end()
        };
        (next - self.frame).clamp(1, RATE / 20)
    }
    /// Preserve the place between sign-in and lobby, and pause it during a duel.
    pub(super) fn advance(&mut self) -> bool {
        self.frame += 1;
        let boundary = self.frame.is_multiple_of(RATE * 4);
        if self.frame == end() {
            self.frame = 0;
            self.cursor = 0;
        }
        boundary
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn complete_editions_fit_the_runtime_and_finish_before_the_loop() {
        assert_eq!(&DATA[..4], b"BAY1");
        assert_eq!(DATA.len(), HEADER + count() * RECORD);
        assert_eq!(count(), 4506); // Every note of all three exported editions.
        let mut previous = 0;
        let mut ends = Vec::new();
        let mut polyphony = 0;
        for note in events() {
            assert!(note.frame >= previous);
            assert!(note.length > 0 && note.frame + note.length + RATE < end());
            assert!((28..=90).contains(&note.pitch));
            assert!(note.gain.is_finite() && note.gain > 0.0 && note.gain < 0.65);
            assert!(note.pan.abs() <= 1.0);
            ends.retain(|&end| end > note.frame);
            ends.push(note.frame + note.length + RATE / 2);
            polyphony = polyphony.max(ends.len());
            previous = note.frame;
        }
        assert!(polyphony < 64, "including release tails: {polyphony}");
        assert!(end() > RATE * 660);
    }
    #[test]
    fn a_loop_restarts_at_the_first_note_after_a_full_release() {
        let mut score = Performance {
            frame: end() - 1,
            cursor: count(),
        };
        score.advance();
        assert_eq!(score.frame, 0);
        assert_eq!(score.cursor, 0);
    }
}
