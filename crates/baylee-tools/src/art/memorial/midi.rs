//! Bounded Standard MIDI reader for the two pinned piano editions.

use anyhow::{Context, Result, bail, ensure};
use std::collections::{BTreeMap, VecDeque};

#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
pub(super) struct Note {
    pub beat: f64,
    pub duration: f64,
    pub part: u16,
    pub pitch: u8,
    pub velocity: u8,
}

struct Reader<'a>(&'a [u8]);
impl<'a> Reader<'a> {
    fn take(&mut self, n: usize) -> Result<&'a [u8]> {
        let bytes = self.0.get(..n).context("truncated MIDI")?;
        self.0 = &self.0[n..];
        Ok(bytes)
    }
    fn byte(&mut self) -> Result<u8> {
        Ok(self.take(1)?[0])
    }
    fn word(&mut self) -> Result<u16> {
        Ok(u16::from_be_bytes(self.take(2)?.try_into()?))
    }
    fn chunk(&mut self, name: &[u8]) -> Result<Reader<'a>> {
        ensure!(self.take(4)? == name, "unexpected MIDI chunk");
        let n = u32::from_be_bytes(self.take(4)?.try_into()?) as usize;
        Ok(Reader(self.take(n)?))
    }
    fn variable(&mut self) -> Result<u32> {
        let mut n = 0;
        for _ in 0..4 {
            let byte = self.byte()?;
            n = (n << 7) | u32::from(byte & 127);
            if byte < 128 {
                return Ok(n);
            }
        }
        bail!("oversized MIDI variable integer")
    }
}

fn track(mut reader: Reader<'_>, part: u16, division: u16, notes: &mut Vec<Note>) -> Result<()> {
    let mut active = BTreeMap::<(u8, u8), VecDeque<(u32, u8)>>::new();
    let mut tick = 0_u32;
    let mut running = 0_u8;
    while !reader.0.is_empty() {
        tick = tick
            .checked_add(reader.variable()?)
            .context("MIDI tick overflow")?;
        let first = reader.byte()?;
        let status = if first >= 128 {
            first
        } else {
            ensure!(
                (0x80..0xf0).contains(&running),
                "invalid MIDI running status"
            );
            running
        };
        if status == 0xff {
            let kind = reader.byte()?;
            let n = reader.variable()? as usize;
            reader.take(n)?;
            if kind == 0x2f {
                ensure!(n == 0 && reader.0.is_empty(), "invalid MIDI end");
            }
            continue;
        }
        if status == 0xf0 || status == 0xf7 {
            running = 0;
            let n = reader.variable()? as usize;
            reader.take(n)?;
            continue;
        }
        ensure!(status < 0xf0, "unsupported MIDI system message");
        running = status;
        let a = if first < 128 { first } else { reader.byte()? };
        let b = if matches!(status >> 4, 0xc | 0xd) {
            0
        } else {
            reader.byte()?
        };
        ensure!(a < 128 && b < 128, "invalid MIDI data byte");
        let key = (status & 15, a);
        if status >> 4 == 9 && b != 0 {
            active.entry(key).or_default().push_back((tick, b));
        } else if matches!(status >> 4, 8 | 9) {
            // LilyPond's Satie edition contains two redundant velocity-zero
            // note-ons. Like MIDI receivers, ignore releases of inactive keys.
            let Some((start, velocity)) = active.get_mut(&key).and_then(VecDeque::pop_front) else {
                continue;
            };
            notes.push(Note {
                beat: f64::from(start) / f64::from(division),
                duration: f64::from(tick - start) / f64::from(division),
                part,
                pitch: a,
                velocity,
            });
        }
    }
    ensure!(
        active.values().all(VecDeque::is_empty),
        "unterminated MIDI note"
    );
    Ok(())
}

pub(super) fn read(bytes: &[u8], repeat: bool) -> Result<Vec<Note>> {
    let mut reader = Reader(bytes);
    let mut header = reader.chunk(b"MThd")?;
    ensure!(header.word()? <= 1, "unsupported MIDI format");
    let tracks = header.word()?;
    let division = header.word()?;
    ensure!(
        division > 0 && division < 0x8000 && header.0.is_empty(),
        "invalid MIDI division/header"
    );
    let mut notes = Vec::new();
    for part in 0..tracks {
        track(reader.chunk(b"MTrk")?, part, division, &mut notes)?;
    }
    ensure!(
        reader.0.is_empty() && !notes.is_empty(),
        "empty or trailing MIDI data"
    );
    if repeat {
        let mut repeated: Vec<_> = notes.iter().copied().filter(|n| n.beat < 117.0).collect();
        repeated.extend(
            notes
                .iter()
                .copied()
                .filter(|n| n.beat < 93.0)
                .map(|mut n| {
                    n.beat += 117.0;
                    n
                }),
        );
        repeated.extend(
            notes
                .iter()
                .copied()
                .filter(|n| n.beat >= 117.0)
                .map(|mut n| {
                    n.beat += 93.0;
                    n
                }),
        );
        notes = repeated;
    }
    notes.sort_by(|a, b| a.partial_cmp(b).expect("integer-derived finite MIDI beats"));
    Ok(notes)
}

#[cfg(test)]
mod tests {
    use super::*;
    const SOURCE: &[u8] =
        include_bytes!("../../../../../art/music/memorial/sources/gymnopedie-1.mid");
    #[test]
    fn written_repeat_and_truncated_input() {
        assert_eq!(read(SOURCE, true).unwrap().len(), 455);
        for cut in 0..SOURCE.len() {
            assert!(read(&SOURCE[..cut], true).is_err(), "cut {cut}");
        }
        assert!(Reader(&[255; 5]).variable().is_err());
        let mut bad = SOURCE.to_vec();
        bad[12..14].copy_from_slice(&0x8000_u16.to_be_bytes());
        assert!(read(&bad, false).is_err());
    }
}
