//! Audition the actual runtime renderer: 40 arrangements and five transition tours.
//! `cargo run --release -p baylee-client-core --example music_demo -- DIR [SECONDS]`
//! Every WAV is stereo PCM24 at the native rate; JSON records peaks and positions.
#![allow(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss
)]
use baylee_client_core::music::{Movement, RATE, SampleSet, ScoreControl, Theme, Tune};
use std::{
    io::{self, Write},
    path::Path,
    sync::Arc,
};

fn wav_header(out: &mut impl Write, frames: u32) -> io::Result<()> {
    let bytes = frames * 6;
    out.write_all(b"RIFF")?;
    out.write_all(&(bytes + 36).to_le_bytes())?;
    out.write_all(b"WAVEfmt ")?;
    out.write_all(&16_u32.to_le_bytes())?;
    out.write_all(&1_u16.to_le_bytes())?;
    out.write_all(&2_u16.to_le_bytes())?;
    out.write_all(&RATE.to_le_bytes())?;
    out.write_all(&(RATE * 6).to_le_bytes())?;
    out.write_all(&6_u16.to_le_bytes())?;
    out.write_all(&24_u16.to_le_bytes())?;
    out.write_all(b"data")?;
    out.write_all(&bytes.to_le_bytes())
}
fn render(
    dir: &Path,
    samples: SampleSet,
    theme: Theme,
    name: &str,
    seconds: u32,
    script: &[(f64, Movement)],
) -> io::Result<serde_json::Value> {
    let control = Arc::new(ScoreControl::default());
    let request = |movement: Movement| {
        let mut r = movement.request(theme);
        r.samples = samples;
        r
    };
    control.set(request(script[0].1));
    let mut tune = Tune::with_control(control.clone());
    let started = std::time::Instant::now();
    let filename = format!("{}-{name}.wav", theme.name());
    let mut out = io::BufWriter::new(std::fs::File::create(dir.join(&filename))?);
    let frames = seconds * RATE;
    wav_header(&mut out, frames)?;
    let mut block = [[0.0_f32; 2]; 256];
    let (mut peak, mut power, mut jump) = (0.0_f32, 0.0_f64, 0.0_f32);
    let mut previous = [0.0_f32; 2];
    let mut changes = Vec::new();
    let mut next = 0;
    let mut done = 0;
    let mut last = tune.position().movement;
    while done < frames {
        let now = f64::from(done) / f64::from(RATE);
        while next < script.len() && script[next].0 <= now {
            control.set(request(script[next].1));
            next += 1;
        }
        let run = (frames - done).min(256) as usize;
        tune.render(&mut block[..run]);
        let position = tune.position();
        if position.movement != last {
            changes.push(serde_json::json!({"seconds": now, "position": position}));
            last = position.movement;
        }
        for frame in &block[..run] {
            for (channel, sample) in frame.iter().enumerate() {
                if !sample.is_finite() || sample.abs() >= 1.0 {
                    return Err(io::Error::other("non-finite or clipping audio"));
                }
                peak = peak.max(sample.abs());
                power += f64::from(*sample).powi(2);
                jump = jump.max((sample - previous[channel]).abs());
                let pcm = (sample * 8_388_607.0).round() as i32;
                out.write_all(&pcm.to_le_bytes()[..3])?;
            }
            previous = *frame;
        }
        done += run as u32;
    }
    out.flush()?;
    let rms_db = 20.0 * (power / f64::from(frames * 2)).sqrt().log10();
    eprintln!("{filename}: peak={peak:.3}, rms={rms_db:.1} dBFS, max-step={jump:.3}");
    Ok(
        serde_json::json!({"file":filename,"samples":samples,"source_rate":44100,"rate":RATE,"bits":24,"seconds":seconds,
        "render_seconds":started.elapsed().as_secs_f64(),"peak":peak,"rms_db":rms_db,"max_step":jump,"transitions":changes,"last_position":tune.position()}),
    )
}
fn main() -> io::Result<()> {
    let args: Vec<_> = std::env::args().collect();
    let dir = Path::new(args.get(1).map_or("/tmp/baylee-dorian", String::as_str));
    let seconds = args
        .get(2)
        .and_then(|v| v.parse::<u32>().ok())
        .unwrap_or(96)
        .clamp(4, 180);
    std::fs::create_dir_all(dir)?;
    let mut report = Vec::new();
    for samples in SampleSet::ALL {
        let dir = &dir.join(samples.name());
        std::fs::create_dir_all(dir)?;
        for theme in Theme::ALL {
            for movement in Movement::ALL {
                report.push(render(
                    dir,
                    samples,
                    theme,
                    movement.name(),
                    seconds,
                    &[(0.0, movement)],
                )?);
            }
            // Deliberately off-beat changes: all routes including combat cancellation
            // and a result dismissed while its attention cue still speaks.
            let tour = [
                (0.0, Movement::Title),
                (7.13, Movement::Lobby),
                (14.27, Movement::Standard),
                (21.41, Movement::Combat),
                (28.53, Movement::Standard),
                (31.79, Movement::Endgame),
                (39.17, Movement::Victory),
                (46.31, Movement::Lobby),
                (49.57, Movement::Defeat),
                (56.81, Movement::Draw),
                (57.07, Movement::Lobby),
            ];
            report.push(render(dir, samples, theme, "transitions", 64, &tour)?);
        }
    }
    std::fs::write(
        dir.join("measurements.json"),
        serde_json::to_string_pretty(&report)?,
    )?;
    Ok(())
}
