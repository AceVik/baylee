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
    for theme in Theme::DORIAN {
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
        // D minor / D Dorian / F major / Handel D-minor Sarabande pitch classes.
        let allowed: &[u8] = if theme == Theme::Glass {
            &[2, 4, 5, 7, 9, 10, 0, 1]
        } else {
            &[2, 4, 5, 7, 9, 10, 11, 0]
        };
        for phrase in [book.tavern, book.table] {
            for &(pitch, _) in phrase.iter().flat_map(|bar| bar.iter()) {
                assert!(
                    pitch == 0 || allowed.contains(&(pitch % 12)),
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
            .filter(|w| w[0].abs_diff(w[1]) >= 5)
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
            .filter(|w| w[0].abs_diff(w[1]) <= 4)
            .count();
        assert!(
            close * 10 > (combat.len() - 1) * 7,
            "{theme:?}: combat moves mostly by small intervals"
        );
    }
}

#[test]
fn all_forty_arrangements_use_the_requested_groups_and_registers() {
    for theme in Theme::DORIAN {
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
            if movement == Movement::Endgame {
                assert!(has(Instrument::Timpani) && has(Instrument::StringPad));
            }
            if movement == Movement::Lobby {
                assert!(!has(Instrument::Trombone));
                assert!(!has(Instrument::Bass));
                assert!(
                    has(Instrument::TavernGuitar) && has(Instrument::Lyre) && has(Instrument::Harp)
                );
            }
            if movement == Movement::Standard {
                assert!(!has(Instrument::Trombone));
                assert!(
                    has(Instrument::TavernGuitar)
                        && has(Instrument::Lyre)
                        && has(Instrument::Viola)
                );
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
            if movement == Movement::Draw && !theme.exploration() {
                assert!(
                    pitches.iter().all(|p| [2, 9].contains(&(p % 12))),
                    "D–A open-fifth suspension: {pitches:?}"
                );
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
    for samples in SampleSet::ALL {
        let control = Arc::new(ScoreControl::default());
        let mut scalar = Tune::with_control(control.clone());
        let mut block = Tune::with_control(control.clone());
        let mut out = [[0.0; 2]; 257];
        for (i, theme) in Theme::ALL.into_iter().enumerate() {
            let mut request = if i == 3 {
                ScoreRequest {
                    scene: Scene::Opening,
                    theme,
                    samples,
                    ..ScoreRequest::default()
                }
            } else {
                let mut r = Movement::ALL[i % 8].request(theme);
                r.samples = samples;
                r
            };
            request.samples = samples;
            control.set(request);
            for round in 0..100 {
                let run = [1, 77, 257, 3, 128][round % 5];
                block.render(&mut out[..run]);
                for frame in &out[..run] {
                    assert_eq!(*frame, scalar.frame());
                }
            }
        }
    }
}

#[test]
fn every_scene_has_finite_audio_and_headroom() {
    for samples in SampleSet::ALL {
        for theme in Theme::ALL {
            for movement in Movement::ALL {
                let control = Arc::new(ScoreControl::default());
                let mut request = movement.request(theme);
                request.samples = samples;
                control.set(request);
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
                assert!(
                    (0.008..0.65).contains(&peak),
                    "{samples:?}/{theme:?}/{movement:?}: peak {peak}"
                );
                assert!(
                    (0.001..0.24).contains(&rms),
                    "{samples:?}/{theme:?}/{movement:?}: rms {rms}"
                );
                assert!(jump < 0.25, "{theme:?}/{movement:?}: {jump}");
            }
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

#[test]
fn a_result_dismissed_before_the_next_pulse_still_gets_its_cue() {
    for theme in Theme::ALL {
        let control = Arc::new(ScoreControl::default());
        control.set(Movement::Standard.request(theme));
        let mut tune = Tune::with_control(control.clone());
        tune.frame();
        control.set(Movement::Draw.request(theme));
        tune.render(&mut [[0.0; 2]; 256]);
        control.set(Movement::Lobby.request(theme));
        let mut out = vec![[0.0; 2]; RATE as usize / 2];
        tune.render(&mut out);
        assert_eq!(
            tune.position.movement,
            Movement::Draw,
            "{theme:?}: cue was lost"
        );
        render_seconds(&mut tune, 3);
        assert_eq!(tune.position.movement, Movement::Lobby);
    }
}

#[test]
fn no_melody_echoes_a_tune_we_must_not() {
    let avoid: serde_json::Value =
        serde_json::from_str(include_str!("../../../../../art/music/avoid.json"))
            .expect("interval signatures");
    let check_intervals = |label: &str, pitches: &[i64]| {
        let ours: Vec<_> = pitches.windows(2).map(|w| w[1] - w[0]).collect();
        for tune in avoid.as_array().expect("avoid list") {
            let mut settings = vec![&tune["intervals"]];
            if let Some(alts) = tune["alt_intervals"].as_array() {
                settings.extend(alts);
            }
            for setting in settings {
                let Some(intervals) = setting.as_array() else {
                    continue;
                };
                let theirs: Vec<_> = intervals
                    .iter()
                    .map(|v| v.as_i64().expect("semitones"))
                    .collect();
                for run in ours.windows(6) {
                    assert!(
                        !theirs.windows(6).any(|other| other == run),
                        "{label}: {} ({run:?})",
                        tune["name"]
                    );
                }
            }
        }
    };

    let mut checked = 0;
    for theme in Theme::ALL {
        let pages = theme.pages();
        for phrase in [
            pages.title,
            pages.answer,
            pages.tavern,
            pages.table,
            pages.combat,
        ] {
            let pitches: Vec<_> = phrase
                .iter()
                .flat_map(|bar| bar.iter())
                .filter(|n| n.0 > 0)
                .map(|n| i64::from(n.0))
                .collect();
            check_intervals(theme.name(), &pitches);
            checked += 1;
        }
    }
    assert_eq!(checked, Theme::ALL.len() * 5);

    for (name, raw_pitches) in memorial::sheet_melodies() {
        let pitches: Vec<_> = raw_pitches.into_iter().map(i64::from).collect();
        check_intervals(name, &pitches);
    }
}

#[test]
fn each_tavern_is_centered_in_d_with_a_distinct_harmonic_route() {
    let mut routes = Vec::new();
    for theme in Theme::DORIAN {
        let mut route = Vec::new();
        let allowed: &[u8] = if theme == Theme::Glass {
            &[2, 4, 5, 7, 9, 10, 0, 1]
        } else {
            &[2, 4, 5, 7, 9, 10, 11, 0]
        };
        for bar in 0..8 {
            let arrangement::Chord(root, third, fifth) =
                arrangement::chord(theme, Movement::Lobby, bar);
            route.push((root, third, fifth));
            for pitch in [root, root + third, root + fifth] {
                assert!(
                    allowed.contains(&(pitch % 12)),
                    "{theme:?} bar {bar}: pitch {pitch}"
                );
            }
        }
        assert_eq!(route[0].0 % 12, 2, "{theme:?} opens on D");
        assert_eq!(route[7].0 % 12, 2, "{theme:?} cadences on D");
        assert!(!routes.contains(&route));
        routes.push(route);
    }
}

#[test]
fn two_bar_themes_return_and_every_written_note_belongs_to_its_harmony() {
    let mut signatures = Vec::new();
    for theme in Theme::DORIAN {
        let pages = theme.pages();
        assert_eq!(
            &pages.title[..2],
            &pages.title[4..6],
            "two-bar thematic return"
        );
        let rhythm: Vec<_> = pages.title[..2]
            .iter()
            .flat_map(|bar| bar.iter().map(|n| n.1))
            .collect();
        assert!(!signatures.contains(&rhythm), "distinct thematic rhythms");
        signatures.push(rhythm);
        for movement in Movement::ALL {
            for bar in 0..33 {
                if movement.ending() && bar == 0 {
                    continue;
                }
                let local = if movement.ending() { bar - 1 } else { bar };
                let arrangement::Chord(root, third, fifth) =
                    arrangement::chord(theme, movement, local);
                assert!(
                    [3, 4].contains(&third) && fifth == 7,
                    "consonant triads only"
                );
                let tones = [root % 12, (root + third) % 12, (root + fifth) % 12];
                for tick in 0..theme.ticks() {
                    for &(stack, intensity) in &[(0, 0.0), (2, 0.55), (5, 1.0)] {
                        let notes = arrangement::notes_with_intensity(
                            theme, movement, bar, tick, stack, intensity,
                        );
                        assert!(notes.as_slice().len() <= 16, "leave transition headroom");
                        for note in notes.as_slice() {
                            assert!(
                                (28..=88).contains(&note.pitch),
                                "{theme:?}/{movement:?} bar {bar} tick {tick}: {note:?}"
                            );
                            assert!(
                                tones.contains(&(note.pitch % 12)),
                                "{theme:?}/{movement:?} bar {bar} tick {tick}: {note:?} vs {tones:?}"
                            );
                        }
                    }
                }
            }
        }
    }
}

#[test]
fn sample_banks_change_without_resetting_the_musical_phrase() {
    for theme in Theme::ALL {
        let control = Arc::new(ScoreControl::default());
        control.set(Movement::Lobby.request(theme));
        let mut tune = Tune::with_control(control.clone());
        render_seconds(&mut tune, 4);
        let before = tune.position();
        let mut request = control.get();
        request.samples = SampleSet::Original441;
        control.set(request);
        let out = render_seconds(&mut tune, 1);
        assert_eq!(tune.position.samples, SampleSet::Original441);
        assert_eq!(tune.position.movement, before.movement);
        assert!(tune.position.phrase_bar >= before.phrase_bar);
        assert!(out.iter().flatten().all(|x| x.is_finite() && x.abs() < 1.0));
        let mut level = crate::music::MusicLevel::default();
        level.set_samples(request.samples);
        let saved = serde_json::to_string(&level).unwrap();
        let restored: crate::music::MusicLevel = serde_json::from_str(&saved).unwrap();
        assert_eq!(restored.samples(), request.samples);
    }
}

#[test]
fn lobby_and_deckbuilder_continue_uninterrupted_on_acoustic_tavern_guitar() {
    let left = Arc::new(ScoreControl::default());
    let right = Arc::new(ScoreControl::default());
    left.set(ScoreRequest {
        scene: Scene::Lobby,
        theme: Theme::Ember,
        ..ScoreRequest::default()
    });
    right.set(ScoreRequest {
        scene: Scene::Lobby,
        theme: Theme::Ember,
        ..ScoreRequest::default()
    });
    let mut a = Tune::with_control(left.clone());
    let mut b = Tune::with_control(right.clone());
    let mut x = [[0.0; 2]; 256];
    let mut y = [[0.0; 2]; 256];
    for i in 0..450 {
        if i == 120 {
            right.set(ScoreRequest {
                scene: Scene::Build,
                theme: Theme::Star,
                ..ScoreRequest::default()
            });
        }
        if i == 280 {
            right.set(ScoreRequest {
                scene: Scene::Lobby,
                theme: Theme::Glass,
                ..ScoreRequest::default()
            });
        }
        a.render(&mut x);
        b.render(&mut y);
        assert_eq!(
            x, y,
            "Lobby <-> Deckbuilder and theme changes never interrupt the Acoustic Tavern Guitar suite"
        );
    }
    assert_eq!(b.position.movement, Movement::Lobby);
    assert_eq!(b.position.theme, Theme::Glass);
}

#[test]
fn table_music_waits_for_the_loading_screen_and_scales_from_calm_to_epic() {
    let control = Arc::new(ScoreControl::default());
    // 1. Lobby plays cozy guitar suite.
    control.set(ScoreRequest {
        scene: Scene::Lobby,
        theme: Theme::Ember,
        ..ScoreRequest::default()
    });
    let mut tune = Tune::with_control(control.clone());
    render_seconds(&mut tune, 2);
    assert_eq!(tune.position().movement, Movement::Lobby);
    assert!(!tune.in_opening_hush());

    // 2. Loading screen (`Scene::Opening`): enters harmonic hush, NEVER starts Table music.
    control.set(ScoreRequest {
        scene: Scene::Opening,
        theme: Theme::Ember,
        ..ScoreRequest::default()
    });
    let hush_audio = render_seconds(&mut tune, 3);
    assert!(tune.in_opening_hush());
    assert_ne!(
        tune.position().movement,
        Movement::Standard,
        "table music must not start while loading screen is active"
    );
    let hush_rms = (hush_audio
        .iter()
        .flatten()
        .map(|s| f64::from(*s).powi(2))
        .sum::<f64>()
        / (hush_audio.len() * 2) as f64)
        .sqrt();
    assert!(
        (0.0002..0.06).contains(&hush_rms),
        "gentle bridge hush: {hush_rms}"
    );

    // 3. Loading screen clears (`Scene::Table`, stack 0): Table music begins at phrase_bar 0!
    control.set(ScoreRequest {
        scene: Scene::Table,
        theme: Theme::Ember,
        stack_depth: 0,
        action_intensity: 0.05,
        ..ScoreRequest::default()
    });
    let mut first_pulse = vec![[0.0; 2]; RATE as usize / 2];
    tune.render(&mut first_pulse);
    assert!(!tune.in_opening_hush());
    assert_eq!(tune.position().movement, Movement::Standard);
    assert_eq!(
        tune.position().phrase_bar,
        0,
        "table theme starts cleanly at bar 0 after loading"
    );
    let calm = render_seconds(&mut tune, 3);
    let bar_before_stack = tune.position().phrase_bar;

    // 4. Stack grows to 2 -> Poised/Tense (`Combat`), phrase_bar continues without resetting!
    control.set(ScoreRequest {
        scene: Scene::Table,
        theme: Theme::Ember,
        stack_depth: 2,
        action_intensity: 0.54,
        tension: 0.35,
        ..ScoreRequest::default()
    });
    render_seconds(&mut tune, 2);
    assert_eq!(tune.position().movement, Movement::Combat);
    assert!(
        tune.position().phrase_bar >= bar_before_stack,
        "phrase-continuous escalation"
    );

    // 5. Stack grows to 4+ -> Epic (`Endgame`), peak remains safe (< 0.65)!
    control.set(ScoreRequest {
        scene: Scene::Table,
        theme: Theme::Ember,
        stack_depth: 5,
        big_spell: true,
        action_intensity: 0.98,
        tension: 0.88,
        ..ScoreRequest::default()
    });
    let epic = render_seconds(&mut tune, 3);
    assert_eq!(tune.position().movement, Movement::Endgame);

    let rms = |buf: &[[f32; 2]]| {
        (buf.iter()
            .flatten()
            .map(|s| f64::from(*s).powi(2))
            .sum::<f64>()
            / (buf.len() * 2) as f64)
            .sqrt()
    };
    let epic_peak = epic
        .iter()
        .flatten()
        .map(|s| s.abs())
        .fold(0.0_f32, f32::max);
    assert!(epic_peak < 0.65, "epic climax peak safe: {epic_peak}");
    assert!(
        rms(&epic) > rms(&calm) * 1.12,
        "epic stack climax swells above calm table: {} vs {}",
        rms(&epic),
        rms(&calm)
    );
}
