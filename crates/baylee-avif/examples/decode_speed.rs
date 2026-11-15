//! How long one frame of a sequence takes to decode on this machine:
//! `cargo run [--release] -p baylee-avif --example decode_speed -- FILE [THREADS] [PASSES]`.
//! Prints keyframe-0, median, mean, p95, p99 and max single-sample latency.

use std::sync::Arc;
use std::time::Instant;

use baylee_avif::{AlignedPlanes, Bounds, Player};

fn main() {
    let mut args = std::env::args().skip(1);
    let Some(path) = args.next() else {
        eprintln!("usage: decode_speed FILE [THREADS] [PASSES]");
        std::process::exit(2);
    };
    let threads: u32 = args.next().and_then(|a| a.parse().ok()).unwrap_or(4);
    let passes: usize = args.next().and_then(|a| a.parse().ok()).unwrap_or(4);
    let bytes: Arc<[u8]> = match std::fs::read(&path) {
        Ok(bytes) => bytes.into(),
        Err(e) => {
            eprintln!("{path}: {e}");
            std::process::exit(1);
        }
    };
    let mut player = match Player::open(bytes, &Bounds::SHIPPED_LOOP, threads) {
        Ok(player) => player,
        Err(e) => {
            eprintln!("{path}: {e}");
            std::process::exit(1);
        }
    };
    let track = &player.sequence().colour;
    println!(
        "{path}: {}×{}, {} frames, {:.3} ms ({} ns), {threads} thread(s)",
        track.width,
        track.height,
        track.samples.len(),
        track.duration_ns() as f64 / 1_000_000.0,
        track.duration_ns(),
    );
    let mut planes = AlignedPlanes::default();
    planes.prepare(track.width, track.height);
    let mut cold_first_ms = 0.0;
    let mut times = Vec::new();
    for pass in 0..passes {
        for idx in 0..player.frames() {
            let start = Instant::now();
            if let Err(e) = player.advance_aligned(&mut planes) {
                eprintln!("frame {}: {e}", player.next_index());
                std::process::exit(1);
            }
            let ms = start.elapsed().as_secs_f64() * 1000.0;
            if pass == 0 && idx == 0 {
                cold_first_ms = ms;
            }
            if pass > 0 || passes == 1 {
                times.push(ms);
            }
        }
    }
    times.sort_by(f64::total_cmp);
    if times.is_empty() {
        return;
    }
    let total: f64 = times.iter().sum();
    #[allow(clippy::cast_precision_loss)] // a count of frames, far below 2^52
    let mean = total / times.len() as f64;
    let pct = |num: usize, den: usize| times[(times.len() * num / den).min(times.len() - 1)];
    println!(
        "cold_f0 {:.2} ms | median {:.2} ms | mean {:.2} ms | p95 {:.2} ms | p99 {:.2} ms | max {:.2} ms ({} frames)",
        cold_first_ms,
        pct(1, 2),
        mean,
        pct(95, 100),
        pct(99, 100),
        times[times.len() - 1],
        times.len()
    );
}
