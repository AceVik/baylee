use super::*;
use arrangement::{Instrument, Role};

#[test]
fn all_fifteen_choices_pack_persist_and_rotate_without_bank_collisions() {
    use crate::music::{MusicLevel, MusicTheme};
    for (index, theme) in Theme::ALL.into_iter().enumerate() {
        assert_eq!(Theme::of(index as u8), theme);
        for samples in SampleSet::ALL {
            let mut request = Movement::Endgame.request(theme);
            request.samples = samples;
            request.turn_seat = 7;
            request.spells = 15;
            let decoded = ScoreRequest::unpack(request.pack());
            assert_eq!(decoded.theme, theme);
            assert_eq!(decoded.samples, samples);
            assert_eq!(decoded.turn_seat, 7);
            assert_eq!(decoded.spells, 15);
            let mut level = MusicLevel::default();
            level.set_theme(MusicTheme::ALL[index]);
            level.set_samples(samples);
            let saved = serde_json::to_string(&level).unwrap();
            let read: MusicLevel = serde_json::from_str(&saved).unwrap();
            assert_eq!(read.theme().pick(0), theme);
            assert_eq!(read.samples(), samples);
        }
        assert_eq!(MusicTheme::Rotating.pick(index as u8), theme);
        assert_eq!(MusicTheme::Rotating.pick(index as u8 + 15), theme);
    }
}
#[test]
fn ten_new_books_have_distinct_motifs_routes_and_palettes() {
    let mut motifs = Vec::new();
    let mut profiles = Vec::new();
    for theme in Theme::EXPLORATIONS {
        let p = styles::profile(theme);
        let book = theme.pages();
        let mut motif = Vec::new();
        let mut last = 0;
        for &(pitch, duration) in book.title[..2].iter().flat_map(|bar| bar.iter()) {
            motif.push((
                if last == 0 || pitch == 0 {
                    0
                } else {
                    i16::from(pitch) - last
                },
                duration,
            ));
            if pitch > 0 {
                last = i16::from(pitch);
            }
        }
        assert!(!motifs.contains(&motif), "{theme:?}: transposed copy");
        motifs.push(motif);
        assert_eq!(
            &book.title[..2],
            &book.title[4..6],
            "{theme:?}: motif return"
        );
        for phrase in [
            book.title,
            book.answer,
            book.tavern,
            book.table,
            book.combat,
        ] {
            for bar in phrase {
                assert_eq!(bar.iter().map(|n| n.1).sum::<u8>(), p.ticks, "{theme:?}");
                assert!(bar.iter().all(|n| n.1 > 0));
            }
        }
        let palette: Vec<_> = (0..p.ticks)
            .flat_map(|tick| {
                arrangement::notes(theme, Movement::Title, 0, tick)
                    .as_slice()
                    .iter()
                    .map(|n| n.instrument as u8)
                    .collect::<Vec<_>>()
            })
            .collect();
        assert!(
            !profiles.contains(&palette),
            "{theme:?}: same orchestration"
        );
        profiles.push(palette);
    }
}
#[test]
fn new_arrangements_stay_consonant_and_inside_the_instrument_and_event_budgets() {
    for theme in Theme::EXPLORATIONS {
        for movement in Movement::ALL {
            for bar in 0..33 {
                if movement.ending() && bar == 0 {
                    continue;
                }
                let local = if movement.ending() { bar - 1 } else { bar };
                let arrangement::Chord(root, third, fifth) =
                    arrangement::chord(theme, movement, local);
                let tones = [root % 12, (root + third) % 12, (root + fifth) % 12];
                for tick in 0..theme.ticks() {
                    let notes = arrangement::notes(theme, movement, bar, tick);
                    assert!(notes.as_slice().len() <= 16, "leave transition headroom");
                    for note in notes.as_slice() {
                        assert!((28..=88).contains(&note.pitch), "{theme:?}: {note:?}");
                        assert!((0.0..=0.6).contains(&note.gain));
                        if note.instrument.percussion() {
                            assert_eq!(note.role, Role::Rhythm);
                        } else {
                            assert!(
                                tones.contains(&(note.pitch % 12)),
                                "{theme:?}/{movement:?} {bar}/{tick}: {note:?} vs {tones:?}"
                            );
                        }
                    }
                }
            }
        }
    }
}
#[test]
fn swing_changes_pulse_lengths_but_not_bar_length_or_response_bound() {
    let control = Arc::new(ScoreControl::default());
    control.set(Movement::Title.request(Theme::Lantern));
    let mut tune = Tune::with_control(control);
    let mut pulses = Vec::new();
    let mut last_tick = tune.tick;
    for frame in 0..RATE * 3 {
        tune.frame();
        if tune.tick != last_tick {
            pulses.push(frame);
            last_tick = tune.tick;
        }
    }
    assert!(pulses[1] - pulses[0] > pulses[2] - pulses[1]);
    let expected = 6.0 * 30.0 / 138.0 * f64::from(RATE);
    assert!((f64::from(pulses[6]) - expected).abs() < 2.0);
    assert!(pulses.windows(2).all(|p| p[1] - p[0] < RATE / 2));
}
#[test]
fn synthetic_only_scores_are_bank_independent_and_acoustic_scores_are_not() {
    for (theme, same) in [(Theme::Circuit, true), (Theme::Copper, false)] {
        let render = |samples| {
            let control = Arc::new(ScoreControl::default());
            let mut request = Movement::Title.request(theme);
            request.samples = samples;
            control.set(request);
            let mut tune = Tune::with_control(control);
            let mut out = vec![[0.0; 2]; RATE as usize * 2];
            tune.render(&mut out);
            out
        };
        assert_eq!(
            render(SampleSet::Studio48) == render(SampleSet::Original441),
            same
        );
    }
    assert!(Instrument::Hat.percussion());
}
