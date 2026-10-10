//! Renders the real score — the shipped sampler, bank and conductor, not a
//! sketch — through every scene and transition, for the owner's ears.
//!
//! ```sh
//! cargo run --release -p baylee-client-core --example music_demo -- <dir>
//! ```
//!
//! Writes one 16-bit stereo WAV per scene or transition into `<dir>` (default
//! `/tmp/baylee-music`), and prints each one's peak and loudness.
#![allow(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss
)] // seconds and frames of a listening demo
use baylee_client_core::music::{RATE, Scene, ScoreControl, ScoreRequest, Theme, Tune};
use std::sync::Mutex;
use std::{io::Write, path::Path, sync::Arc};

/// The theme every request of the next renders sings, and the prefix their
/// files are named with.
static THEME: Mutex<(Theme, &str)> = Mutex::new((Theme::Thorn, ""));

fn with(theme: Theme, prefix: &'static str) {
    *THEME.lock().expect("one thread") = (theme, prefix);
}

/// From `second`, ask for `request`.
type Script = Vec<(f32, ScoreRequest)>;

fn at(scene: Scene, tension: f32) -> ScoreRequest {
    ScoreRequest {
        scene,
        tension,
        ..ScoreRequest::default()
    }
}

fn table(tension: f32) -> ScoreRequest {
    ScoreRequest {
        own_turn: true,
        ..at(Scene::Table, tension)
    }
}

/// A rising line of requests from `from` to `to` tension over `seconds`.
fn ramp(script: &mut Script, start: f32, seconds: f32, from: f32, to: f32, base: ScoreRequest) {
    let steps = (seconds / 2.0) as usize;
    for k in 0..=steps {
        let share = k as f32 / steps as f32;
        script.push((
            start + share * seconds,
            ScoreRequest {
                tension: from + (to - from) * share,
                ..base
            },
        ));
    }
}

fn render(dir: &Path, name: &str, seconds: f32, script: &Script) -> std::io::Result<()> {
    let control = Arc::new(ScoreControl::default());
    let mut tune = Tune::with_control(control.clone());
    let frames = (seconds * RATE as f32) as u32;
    let bytes = frames * 4;
    let (theme, prefix) = *THEME.lock().expect("one thread");
    let path = dir.join(format!("{prefix}{name}.wav"));
    let mut out = std::io::BufWriter::new(std::fs::File::create(&path)?);
    out.write_all(b"RIFF")?;
    out.write_all(&(bytes + 36).to_le_bytes())?;
    out.write_all(b"WAVEfmt ")?;
    out.write_all(&16_u32.to_le_bytes())?;
    out.write_all(&1_u16.to_le_bytes())?;
    out.write_all(&2_u16.to_le_bytes())?;
    out.write_all(&RATE.to_le_bytes())?;
    out.write_all(&(RATE * 4).to_le_bytes())?;
    out.write_all(&4_u16.to_le_bytes())?;
    out.write_all(&16_u16.to_le_bytes())?;
    out.write_all(b"data")?;
    out.write_all(&bytes.to_le_bytes())?;
    let mut next = 0;
    let mut block = [[0.0f32; 2]; 256];
    let (mut peak, mut power) = (0.0f32, 0.0f64);
    let mut done = 0u32;
    while done < frames {
        let now = done as f32 / RATE as f32;
        while next < script.len() && script[next].0 <= now {
            control.set(ScoreRequest {
                theme,
                ..script[next].1
            });
            next += 1;
        }
        let run = (frames - done).min(256) as usize;
        tune.render(&mut block[..run]);
        for frame in &block[..run] {
            for sample in frame {
                peak = peak.max(sample.abs());
                power += f64::from(*sample).powi(2);
                #[allow(clippy::cast_possible_truncation)] // bounded below full scale
                let pcm = (sample * 32767.0).round() as i16;
                out.write_all(&pcm.to_le_bytes())?;
            }
        }
        done += run as u32;
    }
    let rms = (power / f64::from(frames * 2)).sqrt();
    eprintln!(
        "{name}: {seconds:.0} s, peak {peak:.3}, rms {:.1} dBFS",
        20.0 * rms.log10()
    );
    Ok(())
}

#[allow(clippy::too_many_lines)] // one script per preview, read top to bottom
fn main() -> std::io::Result<()> {
    let dir = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "/tmp/baylee-music".into());
    let dir = Path::new(&dir);
    std::fs::create_dir_all(dir)?;
    let started = std::time::Instant::now();
    // The four themes, each at the table (40 s: intimate, the strings
    // swelling in, the horns' statement) and through every scene.
    for (theme, name) in [
        (Theme::Ember, "theme-A-ballad"),
        (Theme::Glass, "theme-B-slavic-dance"),
        (Theme::Thorn, "theme-C-epic-heroic"),
        (Theme::Tide, "theme-D-medieval-jig"),
    ] {
        with(theme, "");
        render(dir, name, 42.0, &vec![(0.0, table(0.05))])?;
        let mut tour: Script = vec![(0.0, at(Scene::Lobby, 0.0))];
        tour.push((
            16.0,
            ScoreRequest {
                arrivals: 1,
                ..at(Scene::Opening, 0.0)
            },
        ));
        tour.push((21.0, table(0.05)));
        ramp(&mut tour, 45.0, 24.0, 0.2, 0.7, table(0.0));
        tour.push((
            70.0,
            ScoreRequest {
                combat: true,
                hunts: 1,
                hunt_mine: true,
                ..table(0.6)
            },
        ));
        tour.push((
            86.0,
            ScoreRequest {
                hunts: 1,
                ..table(0.95)
            },
        ));
        tour.push((
            112.0,
            ScoreRequest {
                hunts: 1,
                ..at(Scene::Victory, 0.0)
            },
        ));
        render(
            dir,
            &format!("{name}-tour-lobby-table-tension-hunt-climax-victory"),
            135.0,
            &tour,
        )?;
        let endings = vec![
            (0.0, table(0.55)),
            (8.0, at(Scene::Draw, 0.0)),
            (34.0, table(0.55)),
            (44.0, at(Scene::Defeat, 0.0)),
        ];
        render(dir, &format!("{name}-draw-then-defeat"), 72.0, &endings)?;
    }
    eprintln!("themes rendered in {:?}", started.elapsed());
    // The scenes as before, in the default theme: the "after" half of the
    // before/after pairs.
    with(Theme::Thorn, "after-");
    scenes(dir)
}

#[allow(clippy::too_many_lines)] // one script per preview, read top to bottom
fn scenes(dir: &Path) -> std::io::Result<()> {
    let started = std::time::Instant::now();

    render(
        dir,
        "01-front-door-bflat-lydian",
        70.0,
        &vec![(0.0, at(Scene::FrontDoor, 0.0))],
    )?;
    render(
        dir,
        "02-lobby-bflat-lydian",
        70.0,
        &vec![(0.0, at(Scene::Lobby, 0.0))],
    )?;
    render(
        dir,
        "03-lobby-to-deck-building-bflat-ionian",
        50.0,
        &vec![(0.0, at(Scene::Lobby, 0.0)), (14.0, at(Scene::Build, 0.0))],
    )?;
    render(
        dir,
        "04-table-opening-arrival-then-c-dorian",
        50.0,
        &vec![
            (0.0, at(Scene::Lobby, 0.0)),
            (
                6.0,
                ScoreRequest {
                    arrivals: 1,
                    ..at(Scene::Opening, 0.0)
                },
            ),
            (12.0, table(0.05)),
        ],
    )?;
    render(
        dir,
        "05-calm-table-c-dorian-own-turn",
        70.0,
        &vec![(0.0, table(0.05))],
    )?;
    render(
        dir,
        "06-calm-table-others-turn",
        50.0,
        &vec![(
            0.0,
            ScoreRequest {
                own_turn: false,
                turn_seat: 1,
                ..table(0.05)
            },
        )],
    )?;
    let mut rising = vec![(0.0, table(0.1))];
    ramp(&mut rising, 10.0, 70.0, 0.15, 0.78, table(0.0));
    render(
        dir,
        "07-calm-to-tension-g-aeolian-pipes-rising",
        85.0,
        &rising,
    )?;
    let hunt = ScoreRequest {
        combat: true,
        ..table(0.5)
    };
    render(
        dir,
        "08-hunt-f-mixolydian-horn-calls",
        70.0,
        &vec![
            (0.0, table(0.4)),
            (
                8.0,
                ScoreRequest {
                    hunts: 1,
                    hunt_mine: true,
                    ..hunt
                },
            ),
            (24.0, table(0.4)),
            (
                34.0,
                ScoreRequest {
                    hunts: 2,
                    hunt_mine: false,
                    own_turn: false,
                    ..hunt
                },
            ),
            (
                52.0,
                ScoreRequest {
                    hunts: 2,
                    ..table(0.4)
                },
            ),
        ],
    )?;
    render(
        dir,
        "09-tension-to-climax-g-aeolian-full-pipes",
        70.0,
        &vec![(0.0, table(0.6)), (10.0, table(0.9))],
    )?;
    let climax = table(0.9);
    for (name, scene, length) in [
        ("10-climax-to-victory-bflat", Scene::Victory, 45.0),
        ("11-tension-to-draw-open-fifth-f-c", Scene::Draw, 45.0),
        ("12-tension-to-defeat-g-aeolian", Scene::Defeat, 45.0),
    ] {
        let before = if scene == Scene::Victory {
            climax
        } else {
            table(0.55)
        };
        render(
            dir,
            name,
            length,
            &vec![
                (0.0, before),
                (12.0, at(scene, 0.0)),
                (38.0, ScoreRequest { ..at(scene, 0.0) }),
            ],
        )?;
    }
    render(
        dir,
        "13-big-spells-bflat-lydian-light",
        45.0,
        &vec![
            (0.0, table(0.1)),
            (
                8.0,
                ScoreRequest {
                    spells: 1,
                    ..table(0.3)
                },
            ),
            (
                20.0,
                ScoreRequest {
                    spells: 1,
                    ..table(0.45)
                },
            ),
            (
                28.0,
                ScoreRequest {
                    spells: 2,
                    ..table(0.45)
                },
            ),
        ],
    )?;
    render(
        dir,
        "14-pipe-drones-recorded-g-then-repitched-f-then-bflat",
        60.0,
        &vec![
            (0.0, table(0.6)),
            (
                20.0,
                ScoreRequest {
                    combat: true,
                    hunts: 1,
                    ..table(0.6)
                },
            ),
            (40.0, at(Scene::Victory, 0.0)),
        ],
    )?;
    render(
        dir,
        "15-monarch-and-about-to-lose",
        40.0,
        &vec![
            (0.0, table(0.4)),
            (
                10.0,
                ScoreRequest {
                    monarchs: 1,
                    ..table(0.4)
                },
            ),
            (
                18.0,
                ScoreRequest {
                    monarchs: 1,
                    about_to_lose: true,
                    ..table(0.6)
                },
            ),
        ],
    )?;
    // A whole game, in five minutes.
    let mut game: Script = vec![
        (0.0, at(Scene::FrontDoor, 0.0)),
        (25.0, at(Scene::Lobby, 0.0)),
        (45.0, at(Scene::Build, 0.0)),
        (65.0, at(Scene::Lobby, 0.0)),
        (
            75.0,
            ScoreRequest {
                arrivals: 1,
                ..at(Scene::Opening, 0.0)
            },
        ),
        (
            81.0,
            ScoreRequest {
                arrivals: 1,
                ..table(0.05)
            },
        ),
        (
            110.0,
            ScoreRequest {
                arrivals: 1,
                own_turn: false,
                turn_seat: 1,
                ..table(0.15)
            },
        ),
    ];
    ramp(
        &mut game,
        130.0,
        30.0,
        0.2,
        0.5,
        ScoreRequest {
            arrivals: 1,
            ..table(0.0)
        },
    );
    game.extend([
        (
            160.0,
            ScoreRequest {
                arrivals: 1,
                combat: true,
                hunts: 1,
                hunt_mine: true,
                ..table(0.55)
            },
        ),
        (
            178.0,
            ScoreRequest {
                arrivals: 1,
                hunts: 1,
                spells: 1,
                ..table(0.5)
            },
        ),
        (
            195.0,
            ScoreRequest {
                arrivals: 1,
                hunts: 1,
                spells: 1,
                ..table(0.7)
            },
        ),
        (
            210.0,
            ScoreRequest {
                arrivals: 1,
                hunts: 1,
                spells: 1,
                ..table(0.9)
            },
        ),
        (
            250.0,
            ScoreRequest {
                arrivals: 1,
                hunts: 1,
                spells: 1,
                ..at(Scene::Victory, 0.0)
            },
        ),
        (
            285.0,
            ScoreRequest {
                arrivals: 1,
                hunts: 1,
                spells: 1,
                ..at(Scene::Lobby, 0.0)
            },
        ),
    ]);
    render(dir, "16-timeline-a-whole-game", 310.0, &game)?;
    eprintln!("rendered in {:?}", started.elapsed());
    Ok(())
}
