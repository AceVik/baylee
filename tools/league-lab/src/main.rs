mod champion_art;
mod competitive;
mod deck;
mod deck_sim;
mod divisions;
mod league;
mod sim;

use sim::Scenario;

const SUITES: [&str; 7] = ["tune", "verify", "fresh", "small", "narrow", "wide", "cap"];

fn suite(name: &'static str) -> Vec<Scenario> {
    let (n, spread, seeds) = match name {
        "pilot" => (4000, 210.0, 0..1),
        "tune" => (8000, 210.0, 10..16),
        "verify" => (12000, 210.0, 100..108),
        "fresh" => (12000, 210.0, 610..618),
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
    if args.is_empty() {
        competitive::run(None);
        return;
    }
    if args.as_slice() == ["ranked-decks"] {
        competitive::deck_trials();
        return;
    }
    if let [cmd, day] = args.as_slice()
        && cmd == "champions"
    {
        if let Err(e) = champion_art::publish(day) {
            eprintln!("{e}");
            std::process::exit(2);
        }
        return;
    }
    if args.as_slice() == ["payouts"] {
        competitive::examples();
        return;
    }
    if args.as_slice() == ["rating-trials"] {
        competitive::rating_trials();
        return;
    }
    if args.first().is_some_and(|s| s == "ranked") && args.len() <= 2 {
        competitive::run(args.get(1).map(String::as_str));
        return;
    }
    let usage = || -> ! {
        eprintln!(
            "Usage: baylee-league-lab ranked [duel|night|town|city|peak|pilot]\n       baylee-league-lab payouts|rating-trials|ranked-decks|ladder|divisions|decks|all|pilot|tune|verify|fresh|small|narrow|wide|cap\n       baylee-league-lab edit REFERENCE_ELO REGISTERED_COPIES CHANGED_COPIES\n       baylee-league-lab champions DAY < accounts.csv"
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
    if action == "ladder" {
        divisions::print_ladder();
        return;
    }
    let divisions = action == "divisions";
    let names = match action.as_str() {
        "all" => SUITES.to_vec(),
        "divisions" => vec!["tune", "fresh"],
        name => vec![
            SUITES
                .into_iter()
                .chain(["pilot"])
                .find(|&n| n == name)
                .unwrap_or_else(|| usage()),
        ],
    };
    if divisions {
        println!(
            "scenario,seed,population,below_gold,from_gold,buffer,band,active_days,up,down,reversals"
        );
    } else {
        println!("{}", sim::header());
    }
    let cases: Vec<_> = names.into_iter().flat_map(suite).collect();
    for batch in cases.chunks(4) {
        std::thread::scope(|scope| {
            let runs: Vec<_> = batch
                .iter()
                .map(|&case| scope.spawn(move || sim::simulate(case, divisions)))
                .collect();
            for run in runs {
                for row in run.join().expect("simulation thread") {
                    println!("{row}");
                }
            }
        });
    }
}
