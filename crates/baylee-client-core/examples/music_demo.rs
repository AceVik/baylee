//! Render the actual runtime orchestra through all states for listening QA.
use baylee_client_core::music::{Mood, RATE, ScoreControl, Tune};
use std::{io::Write, sync::Arc};
fn main() -> std::io::Result<()> {
    let path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "/tmp/baylee-orchestra.wav".into());
    let control = Arc::new(ScoreControl::default());
    let mut tune = Tune::with_control(control.clone());
    let seconds = 150_u32;
    let bytes = seconds * RATE * 4;
    let mut out = std::io::BufWriter::new(std::fs::File::create(path)?);
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
    let started = std::time::Instant::now();
    for frame in 0..seconds * RATE {
        let second = frame / RATE;
        if frame % RATE == 0 {
            let (mood, energy) = match second {
                32..=55 => (Mood::Battle, 0.25),
                56..=79 => (Mood::Battle, 1.0),
                80..=89 => (Mood::Victory, 0.0),
                104..=115 => (Mood::Battle, 0.6),
                116..=125 => (Mood::Defeat, 0.0),
                126..=137 => (Mood::Draw, 0.0),
                _ => (Mood::Sanctuary, 0.0),
            };
            control.set(mood, energy);
        }
        for sample in tune.frame() {
            #[allow(clippy::cast_possible_truncation)] // the sampler is bounded below full scale
            let pcm = (sample * 32767.0).round() as i16;
            out.write_all(&pcm.to_le_bytes())?;
        }
    }
    eprintln!("Rendered {seconds}s in {:?}", started.elapsed());
    Ok(())
}
