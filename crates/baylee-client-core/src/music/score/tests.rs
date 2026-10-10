use super::*;
use arrangement::{Instrument, Role};

fn render_seconds(tune: &mut Tune, seconds: usize) -> Vec<[f32; 2]> {
    let mut out = vec![[0.0; 2]; RATE as usize * seconds];
    tune.render(&mut out);
    out
}

#[test]
fn each_suite_has_complete_distinct_manuscripts() {
    let mut titles = Vec::new();
    for theme in Theme::ALL {
        let book = theme.pages();
        for phrase in [
            book.title,
            book.answer,
            book.tavern,
            book.table,
            book.combat,
        ] {
            for bar in phrase {
                assert_eq!(
                    bar.iter().map(|note| note.1).sum::<u8>(),
                    theme.ticks(),
                    "{theme:?}: {bar:?}"
                );
                assert!(bar.iter().all(|note| note.1 > 0));
            }
        }
        for phrase in [book.tavern, book.table] {
            for &(pitch, _) in phrase.iter().flat_map(|bar| bar.iter()) {
                assert!(
                    pitch == 0 || [10, 0, 1, 3, 5, 7, 8].contains(&(pitch % 12)),
                    "{theme:?}: {pitch}"
                );
            }
        }
        let pitches: Vec<_> = book
            .title
            .iter()
            .flat_map(|bar| bar.iter())
            .filter(|n| n.0 > 0)
            .map(|n| n.0)
            .collect();
        let signature: Vec<_> = pitches
            .windows(2)
            .map(|w| i16::from(w[1]) - i16::from(w[0]))
            .collect();
        assert!(
            !titles.contains(&signature),
            "a re-rhythm is not a new melody"
        );
        titles.push(signature);
        let leaps = pitches
            .windows(2)
            .filter(|w| w[0].abs_diff(w[1]) >= 7)
            .count();
        assert!(
            leaps >= 5,
            "{theme:?}: the title must speak in large intervals"
        );
        let combat: Vec<_> = book
            .combat
            .iter()
            .flat_map(|bar| bar.iter())
            .map(|n| n.0)
            .collect();
        let close = combat
            .windows(2)
            .filter(|w| w[0].abs_diff(w[1]) <= 3)
            .count();
        assert!(
            close * 10 > (combat.len() - 1) * 8,
            "{theme:?}: combat moves mostly by small intervals"
        );
    }
}

#[test]
fn all_forty_arrangements_use_the_requested_groups_and_registers() {
    for theme in Theme::ALL {
        for movement in Movement::ALL {
            let notes: Vec<_> = (0..33)
                .flat_map(|bar| {
                    (0..theme.ticks()).flat_map(move |tick| {
                        arrangement::notes(theme, movement, bar, tick)
                            .as_slice()
                            .to_vec()
                    })
                })
                .collect();
            assert!(!notes.is_empty());
            for note in &notes {
                assert!(note.gain > 0.0 && note.gain <= 0.6);
                assert!(note.length > 0.0);
                assert!(
                    (28..=88).contains(&note.pitch),
                    "{theme:?} {movement:?}: {note:?}"
                );
            }
            let has = |instrument| notes.iter().any(|note| note.instrument == instrument);
            if matches!(movement, Movement::Title | Movement::Endgame) {
                for instrument in [
                    Instrument::Lyre,
                    Instrument::Harp,
                    Instrument::Violin,
                    Instrument::Trombone,
                    Instrument::Bass,
                ] {
                    assert!(has(instrument), "{theme:?} {movement:?}: {instrument:?}");
                }
            }
            if movement == Movement::Lobby {
                assert!(!has(Instrument::Trombone));
                assert!(!has(Instrument::Bass));
                assert!(has(Instrument::Lyre) && has(Instrument::Harp));
            }
            if movement == Movement::Standard {
                assert!(!has(Instrument::Trombone));
                assert!(has(Instrument::Lyre) && has(Instrument::Viola));
            }
            if movement == Movement::Combat {
                let brass = notes
                    .iter()
                    .filter(|n| n.instrument == Instrument::Trombone)
                    .map(|n| n.gain)
                    .fold(0.0_f32, f32::max);
                let lead = notes
                    .iter()
                    .filter(|n| n.role == Role::Melody)
                    .map(|n| n.gain)
                    .fold(0.0_f32, f32::max);
                assert!(brass < lead * 0.35);
                assert!(has(Instrument::ViolaShort) && has(Instrument::CelloShort));
            }
        }
    }
}

#[test]
fn result_cues_have_the_requested_contours_and_do_not_repeat() {
    for theme in Theme::ALL {
        for movement in [Movement::Victory, Movement::Defeat, Movement::Draw] {
            let notes: Vec<_> = (0..theme.ticks())
                .flat_map(|tick| {
                    arrangement::notes(theme, movement, 0, tick)
                        .as_slice()
                        .to_vec()
                })
                .collect();
            let pitches: Vec<_> = notes
                .iter()
                .filter(|n| n.role == Role::Melody)
                .map(|n| n.pitch)
                .collect();
            assert_eq!(pitches.len(), 4);
            if movement == Movement::Victory {
                assert!(pitches.windows(2).all(|w| w[1] > w[0]));
            }
            if movement == Movement::Defeat {
                assert!(pitches.windows(2).all(|w| w[1] < w[0]));
            }
            if movement == Movement::Draw {
                assert!(pitches.windows(2).all(|w| w[1] - w[0] == 4));
            }
            let after: Vec<_> = (0..theme.ticks())
                .flat_map(|tick| {
                    arrangement::notes(theme, movement, 17, tick)
                        .as_slice()
                        .to_vec()
                })
                .collect();
            assert_ne!(
                pitches,
                after
                    .iter()
                    .filter(|n| n.role == Role::Melody)
                    .map(|n| n.pitch)
                    .collect::<Vec<_>>()
            );
        }
    }
}

#[test]
fn every_suite_and_scene_can_enter_within_one_short_pulse() {
    for theme in Theme::ALL {
        let control = Arc::new(ScoreControl::default());
        control.set(Movement::Lobby.request(theme));
        let mut tune = Tune::with_control(control.clone());
        render_seconds(&mut tune, 1);
        let before = tune.position.bar;
        for movement in [
            Movement::Combat,
            Movement::Standard,
            Movement::Endgame,
            Movement::Title,
            Movement::Victory,
        ] {
            control.set(movement.request(theme));
            let mut out = vec![[0.0; 2]; RATE as usize / 2];
            tune.render(&mut out);
            assert_eq!(tune.position.movement, movement, "{theme:?}");
            assert!(out.iter().flatten().all(|sample| sample.is_finite()));
        }
        assert!(tune.position.bar >= before, "global transport survives");
    }
}

#[test]
fn a_dismissed_result_finishes_the_attention_cue_then_flows_out() {
    let control = Arc::new(ScoreControl::default());
    control.set(Movement::Victory.request(Theme::Star));
    let mut tune = Tune::with_control(control.clone());
    tune.frame();
    control.set(Movement::Lobby.request(Theme::Glass));
    let mut out = vec![[0.0; 2]; RATE as usize / 2];
    tune.render(&mut out);
    assert_eq!(tune.position.movement, Movement::Victory);
    render_seconds(&mut tune, 2);
    assert_eq!(tune.position.movement, Movement::Lobby);
    assert_eq!(tune.position.theme, Theme::Glass);
}

#[test]
#[allow(clippy::float_cmp)] // exact scalar/block equivalence is the invariant
fn blocks_and_scalar_frames_match_through_mid_phrase_changes() {
    let control = Arc::new(ScoreControl::default());
    let mut scalar = Tune::with_control(control.clone());
    let mut block = Tune::with_control(control.clone());
    let mut out = [[0.0; 2]; 257];
    for (i, movement) in Movement::ALL.into_iter().enumerate() {
        control.set(movement.request(Theme::ALL[i % 5]));
        for round in 0..100 {
            let run = [1, 77, 257, 3, 128][round % 5];
            block.render(&mut out[..run]);
            for frame in &out[..run] {
                assert_eq!(*frame, scalar.frame());
            }
        }
    }
}

#[test]
fn every_scene_has_finite_audio_and_headroom() {
    for theme in Theme::ALL {
        for movement in Movement::ALL {
            let control = Arc::new(ScoreControl::default());
            control.set(movement.request(theme));
            let mut tune = Tune::with_control(control);
            let frames = render_seconds(&mut tune, 6);
            let mut peak = 0.0_f32;
            let mut power = 0.0_f64;
            let mut jump = 0.0_f32;
            let mut previous = [0.0; 2];
            for frame in &frames {
                for channel in 0..2 {
                    let sample = frame[channel];
                    assert!(sample.is_finite());
                    peak = peak.max(sample.abs());
                    power += f64::from(sample).powi(2);
                    jump = jump.max((sample - previous[channel]).abs());
                }
                previous = *frame;
            }
            let rms = (power / (frames.len() * 2) as f64).sqrt();
            eprintln!("{theme:?}/{movement:?}: peak={peak:.3} rms={rms:.4} jump={jump:.3}");
            assert!(
                (0.008..0.88).contains(&peak),
                "{theme:?}/{movement:?}: {peak}"
            );
            assert!(
                (0.001..0.25).contains(&rms),
                "{theme:?}/{movement:?}: {rms}"
            );
            assert!(jump < 0.25, "{theme:?}/{movement:?}: {jump}");
        }
    }
}

#[test]
fn the_fifth_suite_survives_packing_and_old_settings_migrate() {
    let request = Movement::Endgame.request(Theme::Star);
    assert_eq!(ScoreRequest::unpack(request.pack()), request);
    for (old, new) in [
        ("ballad", super::super::MusicTheme::Ember),
        ("dance", super::super::MusicTheme::Glass),
        ("epic", super::super::MusicTheme::Thorn),
        ("jig", super::super::MusicTheme::Tide),
    ] {
        let level: super::super::MusicLevel =
            serde_json::from_str(&format!(r#"{{"volume":0.7,"theme":"{old}"}}"#)).unwrap();
        assert_eq!(level.theme(), new);
        assert!((level.volume() - 0.7).abs() < 1e-6);
    }
}
