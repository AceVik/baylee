use super::*;
use sha2::{Digest, Sha256};
use std::fmt::Write;

/// The provenance file the bank was prepared from.
const MANIFEST: &str = include_str!("../../../../../art/music/samples.json");
/// The licence policy every source must be named in.
const LEGAL: &str = include_str!("../../../../../docs/legal.md");
/// The three CC0 sources `docs/legal.md` §5 admits, nothing else.
const SOURCES: [&str; 3] = ["VCSL", "VSCO 2 CE", "FreePats Bagpipe"];
/// The bank's ceiling: what it weighs today (19.6 MB at 44.1 kHz, with the
/// orchestral body) and a little room, so a careless addition is noticed
/// rather than shipped.
const CEILING: usize = 20_000_000;

fn rows() -> Vec<serde_json::Value> {
    serde_json::from_str(MANIFEST).expect("samples.json reads")
}

/// Every row of `samples.json` is in the bank, in order, with its bytes
/// embedded and hashing as recorded; every shipped `.pcm` is a row; every
/// row names one of the three CC0 sources at the revision `docs/legal.md`
/// pins; and the whole bank stays under its ceiling. Nothing is fetched at
/// run time: the bytes are in the binary (offline is a hard requirement).
#[test]
fn bank_is_complete_and_embedded() {
    let rows = rows();
    assert_eq!(
        rows.len(),
        bank::BANK.len(),
        "one table row per manifest row"
    );
    let mut total = 0usize;
    for (row, def) in rows.iter().zip(bank::BANK) {
        let name = row["name"].as_str().expect("a name");
        assert_eq!(name, def.name);
        assert_eq!(
            Sha256::digest(def.pcm)
                .iter()
                .fold(String::new(), |mut hex, byte| {
                    let _ = write!(hex, "{byte:02x}");
                    hex
                }),
            row["pcm_sha256"].as_str().expect("a hash"),
            "{name}: the embedded bytes are the prepared ones"
        );
        assert_eq!(
            row["frames"].as_u64(),
            Some(def.pcm.len() as u64 / 2),
            "{name}"
        );
        assert_eq!(row["rate"].as_u64(), Some(u64::from(RATE)), "{name}");
        assert_eq!(row["midi"].as_u64(), Some(u64::from(def.midi)), "{name}");
        #[allow(clippy::float_cmp)] // the table is written from the same number
        {
            assert_eq!(
                row["cents"].as_f64().map(|c| c as f32),
                Some(def.cents),
                "{name}"
            );
        }
        let source = row["source"].as_str().expect("a source");
        assert!(
            SOURCES.contains(&source),
            "{name}: {source} is not a CC0 source we admit"
        );
        let revision = row["revision"].as_str().expect("a revision");
        assert!(
            LEGAL.contains(revision),
            "{name}: docs/legal.md pins {source} at {revision}"
        );
        match (row["loop"].as_array(), def.looped) {
            (None, None) => {}
            (Some(pair), Some((start, end))) => {
                assert_eq!(pair[0].as_u64(), Some(u64::from(start)), "{name}");
                assert_eq!(pair[1].as_u64(), Some(u64::from(end)), "{name}");
                assert_eq!(
                    def.pcm.len() / 2,
                    end as usize + 1,
                    "{name}: the file ends one past the loop"
                );
            }
            other => panic!("{name}: loop {other:?}"),
        }
        total += def.pcm.len();
    }
    assert!(total < CEILING, "the bank weighs {total} bytes");
    assert!(total > CEILING / 2, "the bank was read: {total} bytes");
    for source in SOURCES {
        assert!(LEGAL.contains(source), "docs/legal.md names {source}");
    }
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/orchestra");
    let mut files = 0;
    for entry in std::fs::read_dir(dir).expect("the bank's directory") {
        let path = entry.expect("an entry").path();
        if path.extension().is_some_and(|e| e == "pcm") {
            let stem = path.file_stem().and_then(|s| s.to_str()).expect("a name");
            assert!(
                bank::BANK.iter().any(|def| def.name == stem),
                "{stem}.pcm is shipped and no row"
            );
            files += 1;
        }
    }
    assert_eq!(files, bank::BANK.len());
}

/// The chanter's tuning is the SFZ's, not the measurement's: E5 is the note
/// whose measurement the sketches misread (0.0 against the SFZ's 65 cents
/// flat), and it plays 65 cents sharp of its recording.
#[test]
fn the_chanter_takes_its_tuning_from_the_sfz() {
    let row = rows()
        .into_iter()
        .find(|row| row["name"] == "chanter-E5")
        .expect("the chanter's E5");
    assert_eq!(row["pitch_basis"], "sfz tune");
    assert_eq!(row["cents"].as_f64(), Some(-65.0));
    let measured = row["measured_cents"].as_f64().expect("measured");
    assert!(
        (measured + 65.0).abs() < 15.0,
        "the check agrees: {measured}"
    );
    let e5 = &instruments()[bank::at::CHANTER_E5];
    let semitones = 12.0 * e5.rate(76).log2();
    assert!((semitones - 0.65).abs() < 1e-6, "{semitones}");
}

/// Crossing a loop's seam is no larger a step than the loop's own waveform
/// takes: the crossfade is baked in and the sample after the end is the
/// loop's first.
#[test]
fn a_loop_seam_is_no_jump() {
    let mut looped = 0;
    for (def, instrument) in bank::BANK.iter().zip(instruments()) {
        let Some((start, end)) = instrument.looped else {
            continue;
        };
        looped += 1;
        let body: Vec<f32> = instrument.pcm[start..=end]
            .iter()
            .map(|s| Instrument::decode(*s))
            .collect();
        let mut steps: Vec<f32> = body.windows(2).map(|w| (w[1] - w[0]).abs()).collect();
        steps.sort_by(f32::total_cmp);
        let typical = steps[steps.len() * 99 / 100];
        let seam = (body[0] - body[body.len() - 2]).abs();
        assert!(
            seam <= typical * 2.0 + 0.002,
            "{}: the seam steps {seam}, the loop's 99th percentile {typical}",
            def.name
        );
        assert_eq!(instrument.pcm[end], instrument.pcm[start], "{}", def.name);
    }
    assert!(looped > 40, "{looped} loops checked");
}

/// A looped note outlives its recording: thirty seconds of a recorder held
/// on a two-second sample is still sounding, finite and audible at the end.
#[test]
fn a_held_note_loops_past_its_recording() {
    let mut orchestra = Orchestra::default();
    let alto = bank::ALTO.nearest(74);
    orchestra.note(alto, 74, 30.0, 0.5, Touch::at(0.0));
    let mut late = 0.0f32;
    for frame in 0..RATE as usize * 30 {
        let out = orchestra.frame();
        assert!(out.iter().all(|x| x.is_finite()));
        if frame > RATE as usize * 29 {
            late = late.max(out[0].abs());
        }
    }
    assert_eq!(orchestra.voices(), 1, "still sounding");
    assert!(late > 0.05, "audible at 29 s: {late}");
}

/// The block renderer is the frame renderer, bit for bit: the same notes
/// started at the same frames, rendered both ways over many runs of uneven
/// length, give identical samples — through attacks, releases, recording
/// ends, loop seams, lute strings and the room's whole delay ring.
#[test]
#[allow(clippy::float_cmp)] // bit-identity is the claim
fn a_block_is_the_frames_it_replaces() {
    let mut by_frame = Orchestra::default();
    let mut by_block = Orchestra::default();
    let mut feed = [0.0f32; BLOCK];
    let mut block = [[0.0f32; 2]; BLOCK];
    let runs = [1usize, 7, 256, 13, 100, 256, 3, 199, 256, 64];
    let mut rendered = 0usize;
    for round in 0..1200usize {
        if round % 3 == 0 {
            let instrument = round % bank::BANK.len();
            let pitch = 40 + (round % 40) as u8;
            let seconds = if round % 2 == 0 { 0.05 } else { 2.9 };
            for orchestra in [&mut by_frame, &mut by_block] {
                orchestra.note(instrument, pitch, seconds, 0.3, Touch::at(0.2));
                if round % 9 == 0 {
                    orchestra.lute(43 + (round % 24) as u8, 0.6, 0.2, -0.3, 0.4);
                }
            }
        }
        let run = runs[round % runs.len()];
        by_block.render(&mut block[..run], &mut feed);
        for (k, frame) in block[..run].iter().enumerate() {
            assert_eq!(*frame, by_frame.frame(), "frame {}", rendered + k);
        }
        rendered += run;
    }
    assert!(rendered > 3 * RATE as usize, "{rendered} frames");
}

/// The audio thread never allocates: a minute of notes beyond the polyphony
/// leaves the voice list at the capacity it was built with.
#[test]
fn rendering_never_grows_the_voice_list() {
    let mut orchestra = Orchestra::default();
    let capacity = orchestra.voices.capacity();
    let mut feed = [0.0f32; BLOCK];
    let mut block = [[0.0f32; 2]; BLOCK];
    for round in 0..(60 * RATE as usize / BLOCK) {
        for k in 0..3 {
            orchestra.note(
                (round + k) % bank::BANK.len(),
                60,
                4.0,
                0.05,
                Touch::at(0.0),
            );
        }
        orchestra.lute(50, 1.0, 0.1, 0.0, 0.5);
        orchestra.render(&mut block, &mut feed);
    }
    assert!(orchestra.voices() <= POLYPHONY);
    assert_eq!(orchestra.voices.capacity(), capacity);
}

/// The lute is in tune: its period, all-pass included, is the pitch's.
#[test]
fn the_lute_sounds_its_pitch() {
    for pitch in [43u8, 55, 62, 67] {
        let mut lute = Lute::new();
        lute.pluck(pitch, 1.0, [1.0, 1.0], 0.3, 7);
        let samples: Vec<f32> = (0..RATE as usize / 2)
            .filter_map(|_| lute.next())
            .map(|f| f[0])
            .collect();
        let want = 440.0 * 2.0_f64.powf((f64::from(pitch) - 69.0) / 12.0);
        // The autocorrelation's peak near one period, refined parabolically.
        let period = f64::from(RATE) / want;
        let lag = |l: usize| -> f64 {
            samples[2000..12000]
                .iter()
                .zip(&samples[2000 + l..12000 + l])
                .map(|(a, b)| f64::from(*a) * f64::from(*b))
                .sum()
        };
        let guess = period.round() as usize;
        let best = (guess - 3..=guess + 3)
            .max_by(|a, b| lag(*a).total_cmp(&lag(*b)))
            .expect("a lag");
        let (l, c, r) = (lag(best - 1), lag(best), lag(best + 1));
        let peak = best as f64 + 0.5 * (l - r) / (l - 2.0 * c + r);
        let cents = 1200.0 * (period / peak).log2();
        assert!(cents.abs() < 6.0, "pitch {pitch}: {cents:+.1} cents");
    }
}
