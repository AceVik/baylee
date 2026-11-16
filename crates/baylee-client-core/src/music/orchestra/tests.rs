use super::*;
const ALL: [Instrument; 28] = [
    Instrument::Violin,
    Instrument::Viola,
    Instrument::Cello,
    Instrument::Bass,
    Instrument::Trombone,
    Instrument::Harp,
    Instrument::Zither,
    Instrument::Lyre,
    Instrument::TavernGuitar,
    Instrument::ConcertHarp,
    Instrument::Piano,
    Instrument::Clav,
    Instrument::Flute,
    Instrument::Guitar,
    Instrument::ElectricPiano,
    Instrument::AnalogLead,
    Instrument::ChipLead,
    Instrument::Marimba,
    Instrument::Bell,
    Instrument::Pad,
    Instrument::SynthBass,
    Instrument::FrenchHorn,
    Instrument::StringPad,
    Instrument::Celesta,
    Instrument::Kick,
    Instrument::Snare,
    Instrument::Hat,
    Instrument::Timpani,
];
/// The block renderer is the frame renderer, bit for bit: the same notes
/// started at the same frames, rendered both ways over many runs of uneven
/// length, give identical samples — through attacks, releases, recording
/// ends, loop seams, lute strings, acoustic tavern guitar soundboard modes,
/// hybrid synth layers and the room's whole delay ring.
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
                    orchestra.note(
                        Instrument::TavernGuitar,
                        50 + (round % 24) as u8,
                        0.9,
                        0.22,
                        Touch::at(-0.08).late(0.012),
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
        orchestra.note(Instrument::TavernGuitar, 57, 1.2, 0.1, Touch::at(0.0));
        orchestra.render(&mut block, &mut feed);
    }
    assert!(orchestra.voices.len() <= POLYPHONY);
    assert_eq!(orchestra.voices.capacity(), capacity);
}

/// The lute is in tune: its period, all-pass included, is the pitch's.
#[test]
fn the_lute_sounds_its_pitch() {
    for pitch in [38u8, 43, 50, 55, 62, 67, 74, 81] {
        let mut lute = Lute::new();
        lute.pluck(pitch, 1.0, [1.0, 1.0], 0.3, 0.9986, 7, 0.0);
        let samples: Vec<f32> = (0..RATE as usize * 3 / 4)
            .filter_map(|_| lute.next())
            .map(|f| f[0])
            .collect();
        let want = 440.0 * 2.0_f64.powf((f64::from(pitch) - 69.0) / 12.0);
        let cents = measure_cents_dtft(&samples[2000..24000], want);
        assert!(cents.abs() < 1.0, "pitch {pitch}: {cents:+.3} cents");
    }
}

/// The Acoustic Tavern Guitar is in tune across thumb-plucked Drop-D bass and
/// finger-plucked nylon/gut treble registers, with finite warm body resonance.
#[test]
fn the_tavern_guitar_sounds_its_pitch_and_body_resonances() {
    for pitch in [38u8, 45, 50, 57, 62, 69, 74, 81] {
        let mut guitar = TavernGuitar::new();
        guitar.pluck(pitch, 1.2, [0.8, 0.8], 0.0, 0.0);
        let mut samples = Vec::with_capacity(RATE as usize * 3 / 4);
        for _ in 0..RATE as usize * 3 / 4 {
            let mut dry = [0.0; 2];
            let mut feed = 0.0;
            guitar.frame(&mut dry, &mut feed);
            assert!(dry[0].is_finite() && dry[1].is_finite() && feed.is_finite());
            samples.push(dry[0] + dry[1]);
        }
        let want = 440.0 * 2.0_f64.powf((f64::from(pitch) - 69.0) / 12.0);
        let cents = measure_cents_dtft(&samples[2400..26400], want);
        assert!(cents.abs() < 1.0, "guitar pitch {pitch}: {cents:+.3} cents");
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

/// Fast Hann-windowed phasor-DTFT fundamental pitch estimator (`< 0.01 cent` bias).
fn measure_cents_dtft(samples: &[f32], target_hz: f64) -> f64 {
    let n = samples.len();
    let window: Vec<f64> = (0..n)
        .map(|i| {
            let w = 0.5 * (1.0 - (std::f64::consts::TAU * i as f64 / (n - 1) as f64).cos());
            f64::from(samples[i]) * w
        })
        .collect();
    let power = |cents: f64| -> f64 {
        let freq = target_hz * 2.0_f64.powf(cents / 1200.0);
        let omega = std::f64::consts::TAU * freq / f64::from(RATE);
        let (step_re, step_im) = (omega.cos(), -omega.sin());
        let (mut z_re, mut z_im) = (1.0_f64, 0.0_f64);
        let (mut re, mut im) = (0.0_f64, 0.0_f64);
        for &xw in &window {
            re += xw * z_re;
            im += xw * z_im;
            let next_re = z_re * step_re - z_im * step_im;
            z_im = z_re * step_im + z_im * step_re;
            z_re = next_re;
        }
        re * re + im * im
    };
    let mut best_c = 0.0_f64;
    let mut best_p = -1.0_f64;
    for step in -40..=40 {
        let c = f64::from(step) * 0.75;
        let p = power(c);
        if p > best_p {
            best_p = p;
            best_c = c;
        }
    }
    let (mut lo, mut hi) = (best_c - 0.8, best_c + 0.8);
    let phi = 0.5 * (5.0_f64.sqrt() - 1.0);
    for _ in 0..32 {
        let c1 = hi - phi * (hi - lo);
        let c2 = lo + phi * (hi - lo);
        if power(c1) < power(c2) {
            lo = c1;
        } else {
            hi = c2;
        }
    }
    f64::midpoint(lo, hi)
}

/// Time-domain multi-period normalized autocorrelation (NSDF) pitch estimator in cents.
fn measure_cents_autocorr(samples: &[f32], target_hz: f64) -> f64 {
    let single_period = f64::from(RATE) / target_hz;
    let mult = (1600.0 / single_period).round().clamp(1.0, 24.0);
    let target_lag = single_period * mult;
    let win = 8192.min(samples.len().saturating_sub(target_lag.ceil() as usize + 8));
    let nsdf = |lag: usize| -> f64 {
        let (mut num, mut den) = (0.0_f64, 0.0_f64);
        for i in 0..win {
            let a = f64::from(samples[i]);
            let b = f64::from(samples[i + lag]);
            num += 2.0 * a * b;
            den += a * a + b * b;
        }
        if den > 1e-18 { num / den } else { 0.0 }
    };
    let guess = target_lag.round() as usize;
    let best = (guess.saturating_sub(4)..=guess + 4)
        .max_by(|a, b| nsdf(*a).total_cmp(&nsdf(*b)))
        .expect("non-empty lag search");
    let (l, c, r) = (nsdf(best - 1), nsdf(best), nsdf(best + 1));
    let denom = l - 2.0 * c + r;
    let refined = if denom.abs() > 1e-12 {
        best as f64 + 0.5 * (l - r) / denom
    } else {
        best as f64
    };
    1200.0 * (target_lag / refined).log2()
}

#[test]
fn all_plucked_and_sustained_voices_stay_within_one_cent_across_d2_to_a5() {
    for pitch in 38..=81_u8 {
        let f0 = 440.0 * 2.0_f64.powf((f64::from(pitch) - 69.0) / 12.0);
        let mut lute = Lute::new();
        lute.pluck(pitch, 1.0, [1.0, 1.0], 0.80, 0.9965, 7, 0.0);
        let lute_pcm: Vec<f32> = (0..RATE as usize * 3 / 4)
            .map(|_| lute.next().map_or(0.0, |f| f[0]))
            .collect();
        let c_lute_dtft = measure_cents_dtft(&lute_pcm[2000..24000], f0);
        let c_lute_ac = measure_cents_autocorr(&lute_pcm[2000..24000], f0);
        assert!(
            c_lute_dtft.abs() < 1.0,
            "Lute DTFT {pitch}: {c_lute_dtft:+.3}c"
        );
        assert!(c_lute_ac.abs() < 1.8, "Lute NSDF {pitch}: {c_lute_ac:+.3}c");

        let mut guitar = TavernGuitar::new();
        guitar.pluck(pitch, 1.0, [0.8, 0.8], 0.0, 0.0);
        let mut g_pcm = Vec::with_capacity(RATE as usize * 3 / 4);
        for _ in 0..RATE as usize * 3 / 4 {
            let mut dry = [0.0; 2];
            let mut feed = 0.0;
            guitar.frame(&mut dry, &mut feed);
            g_pcm.push(dry[0] + dry[1]);
        }
        let c_g_dtft = measure_cents_dtft(&g_pcm[2400..26400], f0);
        let c_g_ac = measure_cents_autocorr(&g_pcm[2400..26400], f0);
        assert!(c_g_dtft.abs() < 1.0, "Guitar DTFT {pitch}: {c_g_dtft:+.3}c");
        assert!(c_g_ac.abs() < 1.8, "Guitar NSDF {pitch}: {c_g_ac:+.3}c");
    }
    for inst in [
        Instrument::Piano,
        Instrument::Clav,
        Instrument::Flute,
        Instrument::Guitar,
        Instrument::ElectricPiano,
        Instrument::AnalogLead,
        Instrument::ChipLead,
        Instrument::Pad,
        Instrument::SynthBass,
        Instrument::FrenchHorn,
        Instrument::StringPad,
        Instrument::Celesta,
        Instrument::Timpani,
    ] {
        for pitch in [38_u8, 45, 50, 57, 62, 69, 74, 81] {
            if inst == Instrument::Timpani && pitch > 62 {
                continue;
            }
            let f0 = 440.0 * 2.0_f64.powf((f64::from(pitch) - 69.0) / 12.0);
            let mut s = synth::Synth::default();
            s.note(inst, pitch, 1.2, [0.5, 0.5], Touch::at(0.0));
            let mut pcm = Vec::with_capacity(RATE as usize);
            for _ in 0..RATE as usize {
                let mut out = [0.0; 2];
                let mut send = 0.0;
                s.frame(&mut out, &mut send);
                pcm.push(out[0] + out[1]);
            }
            let win = if inst == Instrument::Timpani {
                &pcm[9600..36000]
            } else {
                &pcm[2400..26400]
            };
            let c_dtft = measure_cents_dtft(win, f0);
            let c_ac = measure_cents_autocorr(win, f0);
            assert!(
                c_dtft.abs() < 1.0 && c_ac.abs() < 1.8,
                "Synth {inst:?} {pitch}: dtft={c_dtft:+.3}c ac={c_ac:+.3}c"
            );
        }
    }
}

#[test]
fn microtonal_just_sweetening_offsets_pitch_accurately_without_drift() {
    for (inst, pitch, target_cents) in [
        (Instrument::Pad, 66_u8, -8.0_f32),
        (Instrument::StringPad, 65, 3.0),
        (Instrument::FrenchHorn, 69, 1.5),
        (Instrument::TavernGuitar, 66, -8.0),
        (Instrument::ConcertHarp, 69, 1.8),
    ] {
        let mut orch = Orchestra::default();
        orch.note(inst, pitch, 1.2, 0.4, Touch::at(0.0).cents(target_cents));
        let pcm: Vec<f32> = (0..RATE as usize * 3 / 4)
            .map(|_| {
                let f = orch.frame();
                assert!(f[0].is_finite() && f[1].is_finite());
                f[0] + f[1]
            })
            .collect();
        let et_hz = 440.0 * 2.0_f64.powf((f64::from(pitch) - 69.0) / 12.0);
        let measured = measure_cents_dtft(&pcm[3600..26000], et_hz);
        assert!(
            (measured - f64::from(target_cents)).abs() < 1.2,
            "{inst:?} wanted {target_cents:+.2}c, got {measured:+.2}c"
        );
    }
}

#[test]
fn the_concert_harp_holds_sub_sixth_cent_waveguide_tuning_and_velvet_rolloff() {
    for pitch in 38..=81_u8 {
        let f0 = 440.0 * 2.0_f64.powf((f64::from(pitch) - 69.0) / 12.0);
        let mut string = plucked::HarpString::new();
        string.pluck(pitch, 1.2, [0.8, 0.8], 0.0, 0.0, 11);
        let raw_pcm: Vec<f32> = (0..RATE as usize * 3 / 4)
            .map(|_| string.next().map_or(0.0, |(pair, _)| pair[0] + pair[1]))
            .collect();
        let raw_cents = measure_cents_dtft(&raw_pcm[2400..28800], f0);
        assert!(
            raw_cents.abs() < 0.15,
            "HarpString {pitch}: {raw_cents:+.4}c >= 0.15c"
        );

        let mut harp = ConcertHarp::new();
        harp.pluck(pitch, 1.2, [0.8, 0.8], 0.0, 0.0);
        let mut body_pcm = Vec::with_capacity(RATE as usize * 3 / 4);
        for _ in 0..RATE as usize * 3 / 4 {
            let mut dry = [0.0; 2];
            let mut feed = 0.0;
            harp.frame(&mut dry, &mut feed);
            assert!(dry[0].is_finite() && dry[1].is_finite() && feed.is_finite());
            body_pcm.push(dry[0] + dry[1]);
        }
        let c_dtft = measure_cents_dtft(&body_pcm[2400..28800], f0);
        let c_ac = measure_cents_autocorr(&body_pcm[2400..28800], f0);
        assert!(
            c_dtft.abs() < 1.0 && c_ac.abs() < 1.8,
            "ConcertHarp {pitch}: dtft={c_dtft:+.3}c ac={c_ac:+.3}c"
        );
    }

    // Velvet finger-pad excitation: partial 7 energy on high D5 (74) must stay < 2% of fundamental.
    let f0 = 440.0 * 2.0_f64.powf((74.0 - 69.0) / 12.0);
    let mut harp = ConcertHarp::new();
    harp.pluck(74, 1.2, [0.8, 0.8], 0.0, 0.0);
    let pcm: Vec<f32> = (0..RATE as usize / 2)
        .map(|_| {
            let mut dry = [0.0; 2];
            let mut feed = 0.0;
            harp.frame(&mut dry, &mut feed);
            dry[0] + dry[1]
        })
        .collect();
    let band_amp = |hz: f64| -> f64 {
        let win = &pcm[2400..16800];
        let (mut re, mut im) = (0.0_f64, 0.0_f64);
        for (i, &s) in win.iter().enumerate() {
            let ang = std::f64::consts::TAU * hz * i as f64 / f64::from(RATE);
            re += f64::from(s) * ang.cos();
            im += f64::from(s) * ang.sin();
        }
        re.hypot(im)
    };
    assert!(
        band_amp(f0 * 7.0) < band_amp(f0) * 0.02,
        "high partials shimmer gently without piercing whistle"
    );
}
