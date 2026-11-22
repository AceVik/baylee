use crate::deck::{self, Pool, Registry};
use rand::{RngExt, SeedableRng, seq::SliceRandom};
use rand_chacha::ChaCha8Rng;

const RUNS: usize = 12000;
const SEED: u64 = 760_019;
const BINS: [&str; 5] = ["0-10%", "10-30%", "30-60%", "60-99%", "100%"];

#[derive(Default)]
struct Sample {
    amount: f64,
    losses: Vec<i32>,
    naive_gaps: Vec<i32>,
    split_gap: i32,
    restore_gap: i32,
    matches: u64,
    recovered: Vec<u16>,
}

fn won(strength: i32, opponent: i32, rng: &mut ChaCha8Rng) -> bool {
    rng.random::<f64>() < 1.0 / (1.0 + 10_f64.powf(f64::from(opponent - strength) / 400.0))
}

fn trial(power: f64, rng: &mut ChaCha8Rng, samples: &mut [Sample; 5]) {
    let size = [60, 75, 100][rng.random_range(0..3)];
    let edits = match rng.random_range(0..10) {
        0..4 => rng.random_range(1..=size / 10),
        4..7 => rng.random_range(size / 10 + 1..=size * 3 / 10),
        7..9 => rng.random_range(size * 3 / 10 + 1..size),
        _ => size,
    };
    let bin = if edits == size {
        4
    } else if edits * 10 <= size {
        0
    } else if edits * 10 <= size * 3 {
        1
    } else if edits * 10 <= size * 6 {
        2
    } else {
        3
    };
    let sample = &mut samples[bin];
    let mut cards: Vec<u32> = (0..size).map(|i| (i / 3) as u32).collect();
    let main = if size == 75 { 60 } else { size };
    let original = Pool::registered(&cards[..main], &cards[main..]);
    let mut positions: Vec<_> = (0..size).collect();
    positions.shuffle(rng);
    let rating = rng.random_range(1300..=2100);
    let mut book = Registry::established(&original, rating, power);
    let mut direct = Registry::established(&original, rating, power);
    let mut naive = rating;
    let mut changed = 0;
    let mut rolled = false;
    let (final_pool, final_start) = loop {
        let step = rng.random_range(1..=6).min(edits - changed);
        for &pos in &positions[changed..changed + step] {
            cards[pos] = 1000 + pos as u32;
        }
        naive = deck::start(naive, step as f64 / size as f64, power);
        changed += step;
        let pool = Pool::registered(&cards[..main], &cards[main..]);
        let entry = book.register(&pool);
        if changed == edits {
            break (pool, entry);
        }
        let opponent = rng.random_range(1250..=1900);
        let win = won(rating, opponent, rng);
        let earned = book.record_result(&pool, opponent, 0, win);
        sample.matches += 1;
        // The naive control re-anchors on the preceding version after each match.
        naive = deck::after_match(naive, opponent, 0, win);
        if !rolled && changed * 2 >= edits {
            book.close_season();
            rolled = true;
        }
        sample.restore_gap = sample
            .restore_gap
            .max((book.register(&original) - rating).abs());
        if size == 75 {
            let mut sideboarded = cards.clone();
            sideboarded.swap(0, 60);
            let key = Pool::registered(&sideboarded[..60], &sideboarded[60..]);
            sample.restore_gap = sample.restore_gap.max((book.register(&key) - earned).abs());
        } else {
            sample.restore_gap = sample
                .restore_gap
                .max((book.register(&pool) - earned).abs());
        }
    };
    let direct_start = direct.register(&final_pool);
    sample.amount += edits as f64 / size as f64;
    sample.losses.push(rating - direct_start);
    sample.naive_gaps.push(naive - direct_start);
    sample.split_gap = sample.split_gap.max((final_start - direct_start).abs());

    // A changed deck's playing strength is independent of the score haircut.
    // Large rebuilds regress toward the field with an unpredictable upgrade/downgrade.
    let q = edits as f64 / size as f64;
    let strength =
        (f64::from(rating) * (1.0 - q) + 1500.0 * q + q * f64::from(rng.random_range(-220..=220)))
            .round() as i32;
    let half = (rating + final_start + 1) / 2;
    let mut first_half = None;
    for game in 1..=120 {
        let opponent = rng.random_range(1250..=1900);
        let current = book.record_result(&final_pool, opponent, 0, won(strength, opponent, rng));
        sample.matches += 1;
        if current >= half {
            first_half.get_or_insert(game);
        }
    }
    if let Some(games) = first_half {
        sample.recovered.push(games);
    }
}

pub fn run() {
    println!(
        "power,edit_share,paths,matches,mean_share,mean_loss,naive_adv_p50,naive_adv_p90,max_split_gap,max_restore_gap,reached_half_by_120,median_games_if_reached"
    );
    for power in [1.5, 2.0, 3.0] {
        let mut rng = ChaCha8Rng::seed_from_u64(SEED);
        let mut samples = std::array::from_fn(|_| Sample::default());
        for _ in 0..RUNS {
            trial(power, &mut rng, &mut samples);
        }
        for (label, mut s) in BINS.iter().zip(samples) {
            s.naive_gaps.sort_unstable();
            s.recovered.sort_unstable();
            let n = s.losses.len();
            assert_eq!((s.split_gap, s.restore_gap), (0, 0));
            let mean_loss = f64::from(s.losses.iter().sum::<i32>()) / n as f64;
            println!(
                "{power},{label},{n},{},{:.5},{mean_loss:.2},{},{},{},{},{},{}",
                s.matches,
                s.amount / n as f64,
                s.naive_gaps[(n - 1) / 2],
                s.naive_gaps[(n - 1) * 9 / 10],
                s.split_gap,
                s.restore_gap,
                s.recovered.len(),
                s.recovered
                    .get(s.recovered.len().saturating_sub(1) / 2)
                    .map_or(String::new(), u16::to_string)
            );
        }
    }
}
