//! Fixed work, no file I/O; prepare outside the measured real-time render.
use baylee_client_core::music::{Movement, RATE, SampleSet, ScoreControl, Theme, Tune};
use std::{hint::black_box, sync::Arc, time::Instant};
fn main() {
    let preparing = Instant::now();
    baylee_client_core::music::prepare();
    println!("prepare_seconds={:.6}", preparing.elapsed().as_secs_f64());
    for samples in SampleSet::ALL {
        for round in 0..3 {
            let start = Instant::now();
            let mut checksum = 0.0_f32;
            for theme in Theme::ALL {
                let control = Arc::new(ScoreControl::default());
                let mut request = Movement::Combat.request(theme);
                request.samples = samples;
                control.set(request);
                let mut tune = Tune::with_control(control);
                let mut block = [[0.0; 2]; 256];
                for _ in 0..(RATE * 30 / 256) {
                    tune.render(black_box(&mut block));
                    checksum += black_box(block[127][0]);
                }
            }
            println!(
                "samples={samples:?} round={round} audio_seconds=150 render_seconds={:.6} checksum={checksum:.6}",
                start.elapsed().as_secs_f64()
            );
        }
    }
}
