//! How long one frame of a sequence takes to decode on this machine:
//! `cargo run --release -p baylee-avif --example decode_speed -- FILE [THREADS] [PASSES]`.
//! Prints the median and the slowest frame of every pass after the first.

use std::sync::Arc;
use std::time::Instant;

use baylee_avif::{Bounds, Frame, Player};

fn main() {
    let mut args = std::env::args().skip(1);
    let Some(path) = args.next() else {
        eprintln!("usage: decode_speed FILE [THREADS] [PASSES]");
        std::process::exit(2);
    };
    let threads: u32 = args.next().and_then(|a| a.parse().ok()).unwrap_or(1);
    let passes: usize = args.next().and_then(|a| a.parse().ok()).unwrap_or(3);
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
        "{path}: {}×{}, {} frames, {} ms, {threads} thread(s)",
        track.width,
        track.height,
        track.samples.len(),
        track.duration_ms()
    );
    let mut frame = Frame::default();
    let mut times = Vec::new();
    for pass in 0..passes {
        for _ in 0..player.frames() {
            let start = Instant::now();
            if let Err(e) = player.advance(&mut frame) {
                eprintln!("frame {}: {e}", player.next_index());
                std::process::exit(1);
            }
            if pass > 0 {
                times.push(start.elapsed().as_secs_f64() * 1000.0);
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
    println!(
        "median {:.2} ms, mean {:.2} ms, slowest {:.2} ms over {} frames",
        times[times.len() / 2],
        mean,
        times[times.len() - 1],
        times.len()
    );
}
