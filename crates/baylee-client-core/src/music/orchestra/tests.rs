use super::*;
const ALL: [Instrument; 8] = [
    Instrument::Violin,
    Instrument::Viola,
    Instrument::Cello,
    Instrument::Bass,
    Instrument::Trombone,
    Instrument::Harp,
    Instrument::Zither,
    Instrument::Lyre,
];
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
    for round in 0..1300usize {
        if round % 3 == 0 {
            let instrument = ALL[round % ALL.len()];
            let pitch = 40 + (round % 40) as u8;
            let seconds = if round % 2 == 0 { 0.05 } else { 2.9 };
            for orchestra in [&mut by_frame, &mut by_block] {
                orchestra.note(instrument, pitch, seconds, 0.3, Touch::at(0.2));
                if round % 9 == 0 {
                    orchestra.note(
                        Instrument::Lyre,
                        43 + (round % 24) as u8,
                        0.6,
                        0.2,
                        Touch::at(-0.3),
                    );
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
    for round in 0..(3 * RATE as usize / BLOCK) {
        for k in 0..3 {
            orchestra.note(ALL[(round + k) % ALL.len()], 60, 4.0, 0.05, Touch::at(0.0));
        }
        orchestra.note(Instrument::Lyre, 50, 1.0, 0.1, Touch::at(0.0));
        orchestra.render(&mut block, &mut feed);
    }
    assert!(orchestra.voices.len() <= POLYPHONY);
    assert_eq!(orchestra.voices.capacity(), capacity);
}

/// The lute is in tune: its period, all-pass included, is the pitch's.
#[test]
fn the_lute_sounds_its_pitch() {
    for pitch in [43u8, 55, 62, 67] {
        let mut lute = Lute::new();
        lute.pluck(pitch, 1.0, [1.0, 1.0], 0.3, 0.9986, 7);
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

#[test]
fn releasing_an_already_releasing_voice_does_not_raise_its_level() {
    let mut orchestra = Orchestra::default();
    orchestra.note(Instrument::Violin, 70, 0.08, 0.4, Touch::at(0.0));
    for _ in 0..RATE / 10 {
        orchestra.frame();
    }
    let before = orchestra.voices[0].level();
    orchestra.release_notes();
    assert!(orchestra.voices[0].level() <= before);
    for _ in 0..RATE {
        orchestra.frame();
    }
    assert!(orchestra.voices.is_empty());
}
