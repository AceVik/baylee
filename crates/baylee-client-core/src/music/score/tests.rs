use super::*;
use crate::music::orchestra::Kind;

/// Whether a recording sounds a pitch: a drum or an unpitched bell does not.
fn pitched(def: &crate::music::orchestra::Def) -> bool {
    match def.kind {
        Kind::Drum => false,
        Kind::Bell => def.name.starts_with("chimes"),
        _ => true,
    }
}

fn table(tension: f32) -> ScoreRequest {
    ScoreRequest {
        scene: Scene::Table,
        tension,
        own_turn: true,
        ..ScoreRequest::default()
    }
}

fn scene(scene: Scene) -> ScoreRequest {
    ScoreRequest {
        scene,
        ..ScoreRequest::default()
    }
}

/// A tune under `control`, already playing `request` (its first bar lines
/// past).
fn settled(request: ScoreRequest, seconds: u32) -> (Arc<ScoreControl>, Tune) {
    let control = Arc::new(ScoreControl::default());
    control.set(request);
    let mut tune = Tune::with_control(control.clone());
    let mut block = [[0.0f32; 2]; BLOCK];
    for _ in 0..(seconds * RATE) as usize / BLOCK {
        tune.render(&mut block);
    }
    (control, tune)
}

/// Renders `seconds` and answers (peak, RMS, largest step between frames).
fn listen(tune: &mut Tune, seconds: u32) -> (f32, f64, f32) {
    let (mut peak, mut power, mut jump) = (0.0f32, 0.0f64, 0.0f32);
    let mut previous = [0.0f32; 2];
    let mut block = [[0.0f32; 2]; BLOCK];
    let frames = (seconds * RATE) as usize / BLOCK * BLOCK;
    for _ in 0..frames / BLOCK {
        tune.render(&mut block);
        for frame in &block {
            for i in 0..2 {
                assert!(frame[i].is_finite());
                peak = peak.max(frame[i].abs());
                power += f64::from(frame[i]).powi(2);
                jump = jump.max((frame[i] - previous[i]).abs());
            }
            previous = *frame;
        }
    }
    (peak, (power / (frames * 2) as f64).sqrt(), jump)
}

fn dbfs(rms: f64) -> f64 {
    20.0 * rms.log10()
}

/// A request is admitted on the next bar line, and until then the present
/// bar plays on exactly as if nothing had been asked; the transport is never
/// reset, and the tempo moves towards the new scene's over seconds.
#[test]
#[allow(clippy::float_cmp)] // exact sample equality proves the request does not touch the current bar
fn requests_wait_for_the_bar_and_never_reset_the_transport() {
    let control = Arc::new(ScoreControl::default());
    let mut tune = Tune::with_control(control.clone());
    let mut reference = Tune::new();
    // Into the second bar, then ask for the table.
    while tune.bar < 1 || tune.tick < 3 {
        assert_eq!(tune.frame(), reference.frame());
    }
    control.set(table(0.0));
    let bar = tune.bar;
    // The bar plays on exactly as the reference does until the next bar
    // line's conductor takes the request.
    loop {
        let (heard, want) = (tune.frame(), reference.frame());
        if tune.pivot.is_some() {
            break;
        }
        assert_eq!(heard, want);
    }
    assert_eq!((tune.bar, tune.tick), (bar + 1, 1), "taken on the bar line");
    assert_eq!(
        tune.texture,
        Texture::FrontDoor,
        "the pivot bar is the front door's"
    );
    assert_eq!(tune.pivot, Some(Texture::Calm));
    for _ in 0..RATE * 6 {
        tune.frame();
    }
    assert_eq!(tune.texture, Texture::Calm);
    assert!(tune.bar > bar + 2, "the bar count runs on");
    assert!(
        tune.eighth < Texture::FrontDoor.eighth(0.0) && tune.eighth > Texture::Calm.eighth(0.0),
        "the tempo slews: {}",
        tune.eighth
    );
}

/// The audio thread's path is the reference path, bit for bit, across
/// scenes, metres, held beds, pipes, the lute and the ticks that schedule
/// every note: the sample iterator and [`Tune::render`] in uneven runs both
/// give exactly what [`Tune::frame`] gives.
#[test]
#[allow(clippy::float_cmp)] // bit-identity is the claim
fn a_rendered_block_is_the_frames_it_replaces() {
    let control = Arc::new(ScoreControl::default());
    let mut reference = Tune::with_control(control.clone());
    let mut streamed = Tune::with_control(control.clone());
    let mut runs = Tune::with_control(control.clone());
    let mut out = [[0.0f32; 2]; BLOCK];
    let lengths = [BLOCK, 1, 77, 255, 3, 128];
    let mut frames = 0usize;
    let script = [
        scene(Scene::Lobby),
        table(0.1),
        table(0.6),
        ScoreRequest {
            combat: true,
            hunts: 1,
            ..table(0.6)
        },
        table(0.95),
        scene(Scene::Victory),
    ];
    for (round, request) in script.into_iter().enumerate() {
        control.set(request);
        for step in 0..500 {
            let run = lengths[(round + step) % lengths.len()];
            runs.render(&mut out[..run]);
            for frame in &out[..run] {
                let want = reference.frame();
                assert_eq!(*frame, want, "frame {frames}");
                let left = streamed.next().expect("endless");
                let right = streamed.next().expect("endless");
                assert_eq!([left, right], want, "streamed frame {frames}");
                frames += 1;
            }
        }
    }
    assert!(frames > 6 * RATE as usize, "{frames} frames");
    assert_eq!(runs.bar, reference.bar);
}

/// The score is deterministic: two performances under the same requests
/// are the same samples, so a replayed game sounds the same.
#[test]
#[allow(clippy::float_cmp)] // sample identity is the claim
fn two_performances_of_one_script_are_identical() {
    let script = [
        (0, scene(Scene::FrontDoor)),
        (10, scene(Scene::Lobby)),
        (
            20,
            ScoreRequest {
                arrivals: 1,
                ..scene(Scene::Opening)
            },
        ),
        (26, table(0.1)),
        (40, table(0.5)),
        (
            55,
            ScoreRequest {
                combat: true,
                hunts: 1,
                ..table(0.6)
            },
        ),
        (
            70,
            ScoreRequest {
                hunts: 1,
                spells: 1,
                ..table(0.9)
            },
        ),
        (90, scene(Scene::Defeat)),
    ];
    let mut a = Tune::with_control(Arc::new(ScoreControl::default()));
    let mut b = Tune::with_control(Arc::new(ScoreControl::default()));
    let mut x = [[0.0f32; 2]; BLOCK];
    let mut y = [[0.0f32; 2]; BLOCK];
    let mut next = 0;
    for block in 0..(120 * RATE as usize / BLOCK) {
        let second = block * BLOCK / RATE as usize;
        while next < script.len() && script[next].0 <= second {
            a.control.set(script[next].1);
            b.control.set(script[next].1);
            next += 1;
        }
        a.render(&mut x);
        b.render(&mut y);
        assert_eq!(x, y, "block {block}");
    }
    assert_eq!(a.texture, Texture::Defeat);
}

/// Every scene is finite, audible, under the headroom (peaks below 0.8, no
/// step between frames a click would make), and as loud as the design asks:
/// the calm table quieter than tension, tension quieter than the climax.
#[test]
fn every_scene_is_finite_audible_and_within_its_loudness() {
    let cases = [
        ("front door", scene(Scene::FrontDoor), -24.0),
        ("lobby", scene(Scene::Lobby), -23.0),
        ("build", scene(Scene::Build), -25.0),
        ("calm", table(0.05), -23.0),
        ("tension", table(0.6), -20.0),
        (
            "hunt",
            ScoreRequest {
                combat: true,
                ..table(0.6)
            },
            -19.0,
        ),
        ("climax", table(0.95), -17.0),
        ("victory", scene(Scene::Victory), -19.0),
        ("draw", scene(Scene::Draw), -21.0),
        ("defeat", scene(Scene::Defeat), -21.0),
    ];
    let mut heard = Vec::new();
    for (name, request, target) in cases {
        let (_, mut tune) = settled(request, 6);
        let (peak, rms, jump) = listen(&mut tune, 12);
        let level = dbfs(rms);
        eprintln!("{name}: peak {peak:.3}, {level:.1} dBFS (target {target}), step {jump:.3}");
        heard.push((name, peak, level, target, jump));
    }
    for &(name, peak, level, target, jump) in &heard {
        assert!(peak < 0.8, "{name}: peak {peak}");
        assert!(jump < 0.25, "{name}: a step of {jump}");
        assert!(
            (level - target).abs() <= 3.0,
            "{name}: {level:.1} dBFS against {target}"
        );
    }
    let heard: Vec<f64> = heard.iter().map(|h| h.2).collect();
    assert!(
        heard[3] < heard[4] && heard[4] < heard[6],
        "calm < tension < climax: {heard:?}"
    );
}

/// The recordings a score note may be played from, and how far: a melody
/// note within a whole tone of its recording, a drone re-pitched by the
/// design's facts (the pipe's G to F and to B♭), a doubling at most a minor
/// third — never a recording dragged out of its own colour.
fn stretched(instrument: usize, pitch: u8) -> u8 {
    bank::BANK[instrument].midi.abs_diff(pitch)
}

/// The score's notes, recorded as it schedules them, over a script that
/// visits every scene, the accents and the transitions between them.
fn performance() -> Vec<Played> {
    let control = Arc::new(ScoreControl::default());
    let mut tune = Tune::with_control(control.clone());
    let script = [
        (0, scene(Scene::FrontDoor)),
        (40, scene(Scene::Lobby)),
        (80, scene(Scene::Build)),
        (
            100,
            ScoreRequest {
                arrivals: 1,
                ..scene(Scene::Opening)
            },
        ),
        (106, table(0.05)),
        (
            150,
            ScoreRequest {
                monarchs: 1,
                spells: 1,
                ..table(0.3)
            },
        ),
        (
            170,
            ScoreRequest {
                monarchs: 1,
                spells: 1,
                ..table(0.6)
            },
        ),
        (
            185,
            ScoreRequest {
                monarchs: 1,
                spells: 2,
                about_to_lose: true,
                ..table(0.75)
            },
        ),
        (
            200,
            ScoreRequest {
                combat: true,
                hunts: 1,
                spells: 2,
                ..table(0.6)
            },
        ),
        (
            230,
            ScoreRequest {
                hunts: 1,
                spells: 3,
                ..table(0.95)
            },
        ),
        (
            300,
            ScoreRequest {
                hunts: 2,
                spells: 3,
                combat: true,
                ..table(0.95)
            },
        ),
        (320, scene(Scene::Victory)),
        (360, table(0.6)),
        (380, scene(Scene::Draw)),
        (420, table(0.6)),
        (440, scene(Scene::Defeat)),
        (480, scene(Scene::Lobby)),
    ];
    let mut block = [[0.0f32; 2]; BLOCK];
    let mut next = 0;
    for k in 0..(500 * RATE as usize / BLOCK) {
        let second = k * BLOCK / RATE as usize;
        while next < script.len() && script[next].0 <= second {
            control.set(script[next].1);
            next += 1;
        }
        tune.render(&mut block);
    }
    std::mem::take(&mut tune.played)
}

/// Every pitched note the score plays is in the set its scene sounds in:
/// the B♭ set (E♭) everywhere, B♭ Lydian's (E♮) at the front door, the lobby
/// and a big spell's lit bar; and every texture was heard.
#[test]
fn every_note_is_in_its_scenes_pitch_set() {
    let played = performance();
    let mut seen = Vec::new();
    for &(instrument, pitch, texture, light, _) in &played {
        if !seen.contains(&texture) {
            seen.push(texture);
        }
        let def = &bank::BANK[instrument];
        let pitched = pitched(def);
        if pitched {
            let lydian = texture.lydian() || light;
            assert!(
                set_holds(lydian, pitch),
                "{texture:?}{}: {} plays MIDI {pitch}, outside its set",
                if light { " (lit)" } else { "" },
                def.name
            );
        }
    }
    for texture in [
        Texture::FrontDoor,
        Texture::Lobby,
        Texture::Build,
        Texture::Arrival,
        Texture::Calm,
        Texture::Tension,
        Texture::Hunt,
        Texture::Climax,
        Texture::Victory,
        Texture::Draw,
        Texture::Defeat,
    ] {
        assert!(seen.contains(&texture), "{texture:?} was never heard");
    }
    assert!(played.iter().any(|p| p.3), "a lit bar was heard");
}

/// No recording is stretched out of its colour: the chanter at most a
/// semitone (its E♭ from E), a melody or bass note a whole tone, a doubling a
/// minor third, the pipe's drones exactly by the design's re-pitching.
#[test]
fn no_recording_is_stretched_out_of_its_colour() {
    for (instrument, pitch, texture, _, _) in performance() {
        let def = &bank::BANK[instrument];
        if !pitched(def) {
            continue;
        }
        let off = stretched(instrument, pitch);
        let most = if def.name.starts_with("chanter") {
            1
        } else {
            3
        };
        assert!(
            off <= most,
            "{texture:?}: {} played at MIDI {pitch}, {off} semitones off",
            def.name
        );
        if def.name.starts_with("drone") {
            assert!([0, 2, 3].contains(&off), "{texture:?}: drone at {pitch}");
        }
    }
}

/// A move of the final waits a bar: the bar after the request is a pivot
/// bar whose bass already plays the new final's fifth and whose melody
/// rests, and the new texture begins on the bar line after it.
#[test]
fn the_drone_moves_at_a_bar_line_through_a_pivot() {
    let (control, mut tune) = settled(table(0.05), 8);
    assert_eq!(tune.texture, Texture::Calm);
    tune.played.clear();
    control.set(table(0.6));
    let asked = tune.bar;
    listen(&mut tune, 6);
    let bars =
        |bar: u64| -> Vec<Played> { tune.played.iter().copied().filter(|p| p.4 == bar).collect() };
    let pivot = bars(asked + 1);
    assert!(
        bars(asked)
            .iter()
            .chain(&pivot)
            .all(|p| p.2 == Texture::Calm),
        "the request's bar and the pivot bar are calm's"
    );
    let bass: Vec<u8> = pivot
        .iter()
        .filter(|p| bank::BANK[p.0].name.starts_with("contrabass"))
        .map(|p| p.1)
        .collect();
    assert_eq!(bass.len(), 1, "one bass note: {bass:?}");
    assert_eq!(bass[0] % 12, (G + 7) % 12, "the bass plays D, G's fifth");
    assert!(
        !pivot.iter().any(|p| ["alto", "violin"]
            .iter()
            .any(|m| bank::BANK[p.0].name.starts_with(m))),
        "the melody rests in the pivot bar"
    );
    let after = bars(asked + 2);
    assert!(
        !after.is_empty() && after.iter().all(|p| p.2 == Texture::Tension),
        "tension begins on the bar line after it"
    );
}

/// An ending finishes its cadence even when its sheet is dismissed at once,
/// then goes where it is asked.
#[test]
fn a_dismissed_ending_finishes_its_cadence_before_returning() {
    let (control, mut tune) = settled(table(0.6), 4);
    control.set(scene(Scene::Victory));
    let mut block = [[0.0f32; 2]; BLOCK];
    while tune.texture != Texture::Victory {
        tune.render(&mut block);
    }
    control.set(scene(Scene::Lobby));
    while tune.here < ENDING_BARS - 1 {
        tune.render(&mut block);
        assert_eq!(tune.texture, Texture::Victory, "bar {}", tune.here);
    }
    for _ in 0..(20 * RATE as usize / BLOCK) {
        tune.render(&mut block);
    }
    assert_eq!(tune.texture, Texture::Lobby);
}

/// The table's hysteresis, bar by bar: tension enters at 0.35 and holds
/// down to 0.2; the climax needs two bars at 0.8 (or a lethal board) and
/// holds down to 0.5; combat is the hunt; a climax of 64 bars breathes.
#[test]
fn the_table_has_hysteresis_and_the_climax_breathes() {
    let mut tune = Tune::new();
    let step = |tune: &mut Tune, request: ScoreRequest| {
        let want = tune.wanted(request);
        if want == tune.texture {
            tune.here += 1;
        } else {
            tune.enter(want);
        }
        tune.texture
    };
    assert_eq!(step(&mut tune, table(0.3)), Texture::Calm);
    assert_eq!(step(&mut tune, table(0.36)), Texture::Tension);
    assert_eq!(
        step(&mut tune, table(0.25)),
        Texture::Tension,
        "holds above 0.2"
    );
    assert_eq!(step(&mut tune, table(0.15)), Texture::Calm);
    assert_eq!(
        step(&mut tune, table(0.85)),
        Texture::Tension,
        "one hot bar is not a climax"
    );
    assert_eq!(step(&mut tune, table(0.85)), Texture::Climax);
    assert_eq!(
        step(&mut tune, table(0.55)),
        Texture::Climax,
        "holds above 0.5"
    );
    assert_eq!(step(&mut tune, table(0.45)), Texture::Tension);
    let lethal = ScoreRequest {
        lethal: true,
        ..table(0.3)
    };
    assert_eq!(
        step(&mut tune, lethal),
        Texture::Climax,
        "a lethal board is the climax at once"
    );
    assert_eq!(
        step(
            &mut tune,
            ScoreRequest {
                combat: true,
                ..table(0.4)
            }
        ),
        Texture::Hunt
    );
    let mut breathed = 0;
    let mut began = false;
    for _ in 0..80 {
        let texture = step(&mut tune, table(0.95));
        began |= texture == Texture::Climax;
        if began && texture == Texture::Tension {
            breathed += 1;
        }
    }
    assert_eq!(
        breathed, BREATH as usize,
        "one breath of {BREATH} bars in 80 bars of climax"
    );
}

/// The pipe is the instrument of rising tension: its drones from 0.25,
/// growing; the chanter from 0.55, already in the tension texture; the
/// climax is full pipes and the davul.
#[test]
fn the_pipe_rises_with_the_tension() {
    let heard = |request: ScoreRequest| {
        let (_, mut tune) = settled(request, 14);
        tune.played.clear();
        listen(&mut tune, 10);
        let names: Vec<&str> = tune.played.iter().map(|p| bank::BANK[p.0].name).collect();
        let level = tune.layers.pipe;
        (names, level)
    };
    let has = |names: &[&str], prefix: &str| names.iter().any(|n| n.starts_with(prefix));
    let (calm, none) = heard(table(0.1));
    assert!(none < 0.01 && !has(&calm, "chanter"), "no pipe at rest");
    let (rising, low) = heard(table(0.3));
    assert!(low > 0.0 && low < 0.3, "the drones enter low: {low}");
    assert!(!has(&rising, "chanter"));
    let (tense, mid) = heard(table(0.6));
    assert!(mid > low, "and grow: {mid}");
    assert!(has(&tense, "chanter"), "the chanter joins at 0.6");
    let (climax, full) = heard(table(0.95));
    assert!((full - 1.0).abs() < 1e-6);
    assert!(has(&climax, "chanter") && has(&climax, "davul"));
}

/// The horn calls on the bar after an attack, once per rest of sixteen
/// bars however often attacks come, never the same call twice running, and
/// only where a hunt can be heard (never in the lobby).
#[test]
fn the_horn_calls_for_the_hunt_and_rests() {
    let horn = |tune: &Tune| {
        tune.played
            .iter()
            .filter(|p| bank::BANK[p.0].name.starts_with("horn"))
            .count()
    };
    let (control, mut tune) = settled(table(0.4), 6);
    let mut block = [[0.0f32; 2]; BLOCK];
    let mut seconds = |tune: &mut Tune, s: u32| {
        for _ in 0..(s * RATE) as usize / BLOCK {
            tune.render(&mut block);
        }
    };
    tune.played.clear();
    control.set(ScoreRequest {
        combat: true,
        hunts: 1,
        hunt_mine: true,
        ..table(0.4)
    });
    seconds(&mut tune, 6);
    let first: Vec<u8> = tune
        .played
        .iter()
        .filter(|p| bank::BANK[p.0].name.starts_with("horn"))
        .map(|p| p.1)
        .collect();
    assert!(first.len() >= 6, "a call of two bars: {first:?}");
    tune.played.clear();
    control.set(ScoreRequest {
        combat: true,
        hunts: 2,
        ..table(0.4)
    });
    seconds(&mut tune, 6);
    assert_eq!(
        horn(&tune),
        0,
        "a second attack within the rest is not called"
    );
    control.set(table(0.4));
    seconds(&mut tune, 30);
    tune.played.clear();
    control.set(ScoreRequest {
        combat: true,
        hunts: 3,
        ..table(0.4)
    });
    seconds(&mut tune, 6);
    let second: Vec<u8> = tune
        .played
        .iter()
        .filter(|p| bank::BANK[p.0].name.starts_with("horn"))
        .map(|p| p.1)
        .collect();
    assert!(!second.is_empty(), "rested, the horn calls again");
    assert_ne!(first, second, "and not the same call");
    let (control, mut lobby) = settled(scene(Scene::Lobby), 2);
    lobby.played.clear();
    control.set(ScoreRequest {
        hunts: 5,
        ..scene(Scene::Lobby)
    });
    seconds(&mut lobby, 6);
    assert_eq!(horn(&lobby), 0, "no hunt in the lobby");
}

/// The melodies are what their metre says: every bar of a 6/8 or 3/4 melody
/// holds six eighths, every 7/8 bar seven; every melody note is in the B♭
/// set (the front door's in its Lydian one).
#[test]
fn every_melody_bar_is_whole_and_in_its_set() {
    for (name, phrase) in MELODIES {
        let seven = name.starts_with("tension");
        let lydian = name.starts_with("front");
        for (k, bar) in phrase.iter().enumerate() {
            let eighths: u8 = bar.iter().map(|n| n.1).sum();
            assert_eq!(eighths, if seven { 7 } else { 6 }, "{name}, bar {k}");
            for &(pitch, _) in *bar {
                assert!(
                    pitch == 0 || set_holds(lydian, pitch),
                    "{name}, bar {k}: {pitch}"
                );
            }
        }
    }
}

/// The interval-signature check (`art/music/originality.py`, design §2.4):
/// no run of six or more directed intervals in any of our melodies matches
/// the opening of a tune on the avoid list, transposed anywhere; and the
/// check fires on an injected opening.
#[test]
fn no_melody_echoes_a_tune_we_must_not() {
    const RUN: usize = 6;
    let avoid: Vec<serde_json::Value> =
        serde_json::from_str(include_str!("../../../../../art/music/avoid.json"))
            .expect("the avoid list reads");
    let intervals = |phrase: Phrase| -> Vec<i32> {
        let pitches: Vec<i32> = phrase
            .iter()
            .flat_map(|bar| bar.iter())
            .filter(|n| n.0 > 0)
            .map(|n| i32::from(n.0))
            .collect();
        pitches.windows(2).map(|w| w[1] - w[0]).collect()
    };
    let shared = |a: &[i32], b: &[i32]| -> usize {
        let mut best = 0;
        for i in 0..a.len() {
            for j in 0..b.len() {
                let mut k = 0;
                while i + k < a.len() && j + k < b.len() && a[i + k] == b[j + k] {
                    k += 1;
                }
                best = best.max(k);
            }
        }
        best
    };
    let steps = |value: &serde_json::Value| -> Vec<i32> {
        value
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(serde_json::Value::as_i64)
            .map(|v| v as i32)
            .collect()
    };
    // Every setting of every opening: the primary and the alternates the
    // sources disagree on.
    let tunes: Vec<(String, Vec<i32>)> = avoid
        .iter()
        .flat_map(|tune| {
            let name = tune["name"].as_str().unwrap_or("?").to_owned();
            std::iter::once(steps(&tune["intervals"]))
                .chain(
                    tune["alt_intervals"]
                        .as_array()
                        .into_iter()
                        .flatten()
                        .map(steps),
                )
                .filter(|s| !s.is_empty())
                .map(move |s| (name.clone(), s))
        })
        .collect();
    assert!(
        avoid.iter().filter(|tune| tune["verified"] == true).count() >= 17,
        "the verified openings were read"
    );
    assert!(
        tunes.len() >= 22,
        "the avoid list was read: {}",
        tunes.len()
    );
    for (name, phrase) in MELODIES {
        let ours = intervals(phrase);
        for (tune, theirs) in &tunes {
            let run = shared(&ours, theirs);
            assert!(run < RUN, "{name} shares {run} intervals with {tune}");
        }
    }
    let (tune, theirs) = &tunes[0];
    let injected = theirs[..RUN.min(theirs.len())].to_vec();
    assert!(
        shared(&injected, theirs) >= RUN.min(theirs.len()),
        "the check fires on {tune}"
    );
}
