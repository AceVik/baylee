//! Authored rubato, phrasing, half-pedal and restrained string responses.
#![allow(clippy::cast_precision_loss)]

use super::{HERE, RATE, midi};
use crate::art::samples::sha256;
use anyhow::{Context, Result, ensure};
use serde_json::Value;
use std::{collections::BTreeMap, path::Path};

fn interpolate(points: &[(f64, f64)], x: f64) -> f64 {
    for pair in points.windows(2) {
        let [(a, va), (b, vb)] = [pair[0], pair[1]];
        if x <= b {
            return va + (vb - va) * ((x - a) / (b - a)).clamp(0.0, 1.0);
        }
    }
    points.last().expect("nonempty authored map").1
}

fn phrasing(satie: bool, beat: f64) -> (f64, f64) {
    type Curve = &'static [(f64, f64)];
    let bar = beat / if satie { 3.0 } else { 4.5 };
    let (tempo, dynamics): (Curve, Curve) = if satie {
        (
            &[
                (0., 72.),
                (20., 74.),
                (30., 70.),
                (38., 60.),
                (40., 72.),
                (59., 74.),
                (70., 70.),
                (77., 58.),
                (78., 48.),
            ],
            &[
                (0., 51.),
                (5., 61.),
                (12., 50.),
                (21., 65.),
                (29., 58.),
                (35., 67.),
                (39., 46.),
                (44., 61.),
                (57., 57.),
                (65., 69.),
                (72., 61.),
                (78., 36.),
            ],
        )
    } else {
        (
            &[
                (0., 65.),
                (13., 65.),
                (16., 59.),
                (25., 63.),
                (27., 77.),
                (35., 81.),
                (39., 83.),
                (42., 69.),
                (49., 68.),
                (51., 64.),
                (64., 63.),
                (69., 55.),
                (72., 46.),
            ],
            &[
                (0., 54.),
                (7., 62.),
                (13., 57.),
                (17., 72.),
                (22., 63.),
                (26., 68.),
                (31., 79.),
                (37., 88.),
                (40., 94.),
                (43., 65.),
                (49., 61.),
                (51., 53.),
                (63., 48.),
                (69., 41.),
                (72., 34.),
            ],
        )
    };
    let phase = (bar % 4.0) / 4.0;
    (
        interpolate(tempo, bar) * (1.0 - 0.045 * (-((phase - 0.96) / 0.11).powi(2)).exp()),
        interpolate(dynamics, bar),
    )
}

#[derive(PartialEq, PartialOrd)]
struct Event(f64, f64, f32, f32, u8, u8, u8);

fn clock(satie: bool, end: f64) -> Vec<f64> {
    let mut grid = vec![0.0];
    let mut elapsed = 0.0;
    for i in 0..=((end + 6.0) * 192.0).ceil() as usize {
        elapsed += 60.0 / phrasing(satie, (i as f64 + 0.5) / 192.0).0 / 192.0;
        grid.push(elapsed);
    }
    grid
}

fn voice_extremes(notes: &[midi::Note]) -> (BTreeMap<u64, u8>, BTreeMap<u64, u8>) {
    // Beats come from nonnegative integer ticks; their exact bit patterns
    // identify simultaneous onsets without a floating-point tolerance.
    let mut tops = BTreeMap::<u64, u8>::new();
    let mut lows = BTreeMap::<u64, u8>::new();
    for n in notes {
        if n.part == 1 {
            tops.entry(n.beat.to_bits())
                .and_modify(|p| *p = (*p).max(n.pitch))
                .or_insert(n.pitch);
        }
        if n.part == 2 {
            lows.entry(n.beat.to_bits())
                .and_modify(|p| *p = (*p).min(n.pitch))
                .or_insert(n.pitch);
        }
    }
    (tops, lows)
}

fn perform(notes: &[midi::Note], satie: bool, offset: f64, events: &mut Vec<Event>) -> f64 {
    let end = notes
        .iter()
        .map(|n| n.beat + n.duration)
        .fold(0.0, f64::max);
    let grid = clock(satie, end);
    let at = |beat: f64| {
        let pos = (beat * 192.0).max(0.0);
        let i = pos as usize;
        grid[i] + (grid[i + 1] - grid[i]) * (pos - i as f64)
    };
    let (tops, lows) = voice_extremes(notes);
    let onsets: Vec<_> = notes
        .iter()
        .filter(|n| n.part == 2)
        .map(|n| n.beat)
        .collect();
    for (i, note) in notes.iter().enumerate() {
        let midi::Note {
            beat,
            duration,
            part,
            pitch,
            velocity: source_velocity,
        } = *note;
        let start = at(beat);
        let stop = at(beat + duration);
        let melody = part == 1 && tops.get(&beat.to_bits()) == Some(&pitch);
        let mut velocity = phrasing(satie, beat).1
            + if melody {
                8.0
            } else if part == 1 {
                -8.0
            } else {
                -12.0
            };
        velocity += 2.0 * (i as f64 * 1.731).sin();
        if satie && melody {
            velocity += (f64::from(source_velocity) - 75.0) * 0.10;
        }
        let velocity = velocity.clamp(29.0, 104.0).round_ties_even() as u8;
        let next = onsets.partition_point(|onset| *onset <= beat + 0.03);
        let pedal = at(onsets.get(next).copied().unwrap_or(end));
        let hold = stop.max(pedal.min(stop + if satie { 0.9 } else { 0.65 }));
        let late = if melody { 0.010 } else { 0.002 } + 0.003 * (i as f64 * 2.17).sin();
        events.push(Event(
            offset + start + late,
            hold - start,
            0.60,
            0.0,
            0,
            pitch,
            velocity,
        ));
        let bar = beat / if satie { 3.0 } else { 4.5 };
        let violin = if satie {
            (43.0..73.0).contains(&bar)
        } else {
            (14.0..26.0).contains(&bar) || (43.0..50.0).contains(&bar)
        };
        if melody && duration >= 0.85 && violin && (55..=88).contains(&pitch) {
            events.push(Event(
                offset + start + 0.025,
                (stop - start - 0.03).max(0.15),
                0.105,
                -0.22,
                1,
                pitch,
                velocity,
            ));
        }
        let cello = if satie {
            (39.0..76.0).contains(&bar)
        } else {
            (14.0..50.0).contains(&bar)
        };
        let bass = part == 2 && lows.get(&beat.to_bits()) == Some(&pitch);
        if bass && duration >= 1.7 && cello && (36..=59).contains(&pitch) {
            events.push(Event(
                offset + start + 0.018,
                (stop - start).min(5.5),
                0.085,
                0.25,
                2,
                pitch,
                velocity.max(35),
            ));
        }
    }
    at(end)
}

pub(super) fn prepare(root: &Path) -> Result<Vec<u8>> {
    let here = root.join(HERE);
    let sources: Vec<Value> = serde_json::from_slice(&std::fs::read(here.join("sources.json"))?)?;
    for work in sources {
        for source in work["files"].as_array().context("source files")? {
            let path = source["path"].as_str().context("source path")?;
            ensure!(
                Path::new(path).components().count() == 1
                    && Path::new(path)
                        .components()
                        .all(|c| matches!(c, std::path::Component::Normal(_))),
                "invalid score path"
            );
            ensure!(
                sha256(&std::fs::read(here.join("sources").join(path))?) == source["sha256"],
                "source hash mismatch: {path}"
            );
        }
    }
    let mut events = Vec::new();
    let mut offset = 0.65;
    for (work, satie, count) in [("clair-de-lune", false, 1468), ("gymnopedie-1", true, 455)] {
        let notes = midi::read(
            &std::fs::read(here.join("sources").join(format!("{work}.mid")))?,
            satie,
        )?;
        ensure!(notes.len() == count, "source note count changed");
        offset += perform(&notes, satie, offset, &mut events) + 7.0;
    }
    events.sort_by(|a, b| a.partial_cmp(b).expect("finite authored events"));
    let mut bytes = b"BAY2".to_vec();
    bytes.extend(u32::try_from(events.len())?.to_le_bytes());
    bytes.extend(((offset * f64::from(RATE)).round_ties_even() as u32).to_le_bytes());
    for Event(start, length, gain, pan, instrument, pitch, velocity) in events {
        for time in [start, length] {
            bytes.extend(((time * f64::from(RATE)).round_ties_even() as u32).to_le_bytes());
        }
        bytes.extend(gain.to_le_bytes());
        bytes.extend(pan.to_le_bytes());
        bytes.extend([instrument, pitch, velocity, 0]);
    }
    let report: Value = serde_json::from_slice(&std::fs::read(here.join("performance.json"))?)?;
    ensure!(
        sha256(&bytes) == report["sha256"],
        "authored performance hash mismatch"
    );
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    #[test]
    fn pinned_scores_reproduce_every_performance_byte() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        assert_eq!(
            super::prepare(&root).unwrap(),
            std::fs::read(root.join(super::super::SCORE)).unwrap()
        );
    }
}
