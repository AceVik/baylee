mod deck;
mod deck_sim;
mod league;
mod sim;

use sim::Scenario;

const SUITES: [&str; 6] = ["tune", "verify", "small", "narrow", "wide", "cap"];

fn suite(name: &'static str) -> Vec<Scenario> {
    let (n, spread, seeds) = match name {
        "pilot" => (4000, 210.0, 0..1),
        "tune" => (8000, 210.0, 10..16),
        "verify" => (12000, 210.0, 100..108),
        "small" => (800, 210.0, 200..204),
        "narrow" => (12000, 165.0, 300..303),
        "wide" => (12000, 255.0, 400..403),
        "cap" => (150_000, 255.0, 500..501),
        _ => unreachable!(),
    };
    seeds
        .map(|seed| Scenario {
            label: name,
            population: n,
            spread,
            seed,
        })
        .collect()
}

fn main() {
    let args: Vec<_> = std::env::args().skip(1).collect();
    let usage = || -> ! {
        eprintln!(
            "Usage: baylee-league-lab all|pilot|tune|verify|small|narrow|wide|cap|decks\n       baylee-league-lab edit REFERENCE_ELO REGISTERED_COPIES CHANGED_COPIES"
        );
        std::process::exit(2)
    };
    if let [cmd, rating, total, changed] = args.as_slice()
        && cmd == "edit"
    {
        let rating: i32 = rating.parse().unwrap_or_else(|_| usage());
        let total: u32 = total.parse().unwrap_or_else(|_| usage());
        let changed: u32 = changed.parse().unwrap_or_else(|_| usage());
        if rating < deck::FLOOR || total == 0 || changed > total {
            usage();
        }
        let score = deck::start(rating, f64::from(changed) / f64::from(total), 2.0);
        println!("{rating} -> {score} Elo ({} lost)", rating - score);
        return;
    }
    let [action] = args.as_slice() else { usage() };
    if action == "decks" {
        deck_sim::run();
        return;
    }
    let names = match action.as_str() {
        "all" => SUITES.to_vec(),
        name => vec![
            SUITES
                .into_iter()
                .chain(["pilot"])
                .find(|&n| n == name)
                .unwrap_or_else(|| usage()),
        ],
    };
    println!("{}", sim::header());
    let cases: Vec<_> = names.into_iter().flat_map(suite).collect();
    for batch in cases.chunks(4) {
        std::thread::scope(|scope| {
            let runs: Vec<_> = batch
                .iter()
                .map(|&case| scope.spawn(move || sim::simulate(case)))
                .collect();
            for run in runs {
                for row in run.join().expect("simulation thread") {
                    println!("{row}");
                }
            }
        });
    }
}
