use super::*;

#[test]
fn studio_cache_is_bounded_and_covers_every_composed_pitch() {
    let mut count = 0;
    let mut bytes = 0;
    for clip in clips().iter().flatten() {
        count += 1;
        bytes += clip.data.len() * 4;
        assert!(clip.data.iter().all(|x| x.is_finite() && x.abs() < 1.2));
        if let Some((start, end)) = clip.looped {
            assert!(start < end && end == clip.data.len());
            assert!((clip.data[start] - clip.data[end - 1]).abs() < 0.15);
        }
    }
    eprintln!("studio cached {count} pitched clips / {bytes} bytes");
    assert!((50..350).contains(&count));
    assert!(bytes < 110_000_000);
    for theme in Theme::ALL {
        for movement in Movement::ALL {
            for bar in 0..33 {
                for tick in 0..theme.ticks() {
                    for note in arrangement::notes(theme, movement, bar, tick).as_slice() {
                        if note.instrument != Instrument::Lyre {
                            assert!(clips()[key(note.instrument, note.pitch)].is_some());
                        }
                    }
                }
            }
        }
    }
}
#[test]
fn sinc_passband_is_unity_and_downsampling_rejects_aliases() {
    for step in [0.6, 1.0, 1.5, 2.0] {
        for coefficients in kernel(step) {
            assert!((coefficients.iter().sum::<f32>() - 1.0).abs() < 0.000_001);
            // At 1.5x and 2x, energy near source Nyquist must not fold back.
            if step >= 1.5 {
                let nyquist: f32 = coefficients
                    .iter()
                    .enumerate()
                    .map(|(i, x)| if i % 2 == 0 { *x } else { -x })
                    .sum();
                assert!(nyquist.abs() < 0.001);
            }
        }
    }
}
#[test]
fn bank_pcm_matches_the_pinned_licensed_manifest() {
    use sha2::{Digest, Sha256};
    use std::fmt::Write;
    let rows: serde_json::Value =
        serde_json::from_str(include_str!("../../../../../../art/music/samples.json")).unwrap();
    for instrument in [
        Instrument::Harp,
        Instrument::Zither,
        Instrument::Violin,
        Instrument::Viola,
        Instrument::Cello,
        Instrument::Bass,
        Instrument::Trombone,
        Instrument::ViolinShort,
        Instrument::ViolaShort,
        Instrument::CelloShort,
    ] {
        let family = family(instrument);
        for def in &bank::BANK[family.first..family.first + family.len] {
            let row = rows
                .as_array()
                .unwrap()
                .iter()
                .find(|row| row["name"] == def.name)
                .unwrap();
            assert_eq!(row["rate"], SOURCE_RATE);
            let revision = match row["source"].as_str().unwrap() {
                "VCSL" => "c1ea7bcc3c7309650ab0da9d15c9cd1fbc4a4c7e",
                "VSCO 2 CE" => "440300901dfe9275fd84e0b7763af1f8443ae62e",
                other => panic!("unexpected source: {other}"),
            };
            assert_eq!(row["revision"], revision);
            assert_eq!(
                row["pcm_sha256"],
                Sha256::digest(def.pcm)
                    .iter()
                    .fold(String::with_capacity(64), |mut hex, byte| {
                        write!(hex, "{byte:02x}").unwrap();
                        hex
                    })
            );
        }
    }
}


#[test]
fn studio_pitch_conversion_matches_a_known_sine_without_phase_steps() {
    let pcm: Vec<u8> = (0..SOURCE_RATE).flat_map(|i| {
        let wave = (std::f64::consts::TAU*440.0*f64::from(i)/f64::from(SOURCE_RATE)).sin()*0.4;
        ((wave*32767.0).round() as i16).to_le_bytes()
    }).collect();
    let def = Def { name: "test-sine", pcm: Box::leak(pcm.into_boxed_slice()),
        midi: 69, cents: 0.0, looped: None, kind: Kind::Pluck,
        attack: 0.0, release: 0.1, room_send: 0.0 };
    for pitch in [57,70,81] {
        let clip = prepare_clip(&def,pitch);
        let frequency = 440.0*2.0_f64.powf((f64::from(pitch)-69.0)/12.0);
        let error = (1000..11000).map(|i| {
            let expected = (std::f64::consts::TAU*frequency*i as f64/f64::from(RATE)).sin()*0.4;
            (f64::from(clip.data[i])-expected).powi(2)
        }).sum::<f64>()/10000.0;
        assert!(error.sqrt()<0.0001, "pitch {pitch}: rms error {}", error.sqrt());
    }
}
