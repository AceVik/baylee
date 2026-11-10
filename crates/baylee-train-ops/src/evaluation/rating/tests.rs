use super::games::{Game, Match};
use super::*;
use crate::evaluation::sample;
use std::path::Path;

fn options(seed: u64) -> Options {
    Options {
        resamples: 200,
        seed,
        thin_link_games: 200.0,
    }
}

fn uniform(state: &mut u64) -> f64 {
    sample(state, 1 << 30) as f64 / f64::from(1_u32 << 30)
}

const DECKS: [&str; 12] = ["a", "b", "c", "d", "e", "f", "g", "h", "i", "j", "k", "l"];

/// `games` games of `player` against `opponent`, rated `difference` above
/// it, drawn from the model itself; two seatings of a deal share their decks.
fn played(player: &str, opponent: &str, difference: f64, games: usize, state: &mut u64) -> Match {
    let expected = fit::expected(difference);
    let games: Vec<Game> = (0..games)
        .map(|i| Game {
            score: if uniform(state) < expected { 1.0 } else { 0.0 },
            deal: i / 2,
            decks: vec![
                DECKS[(i / 2) % DECKS.len()].to_owned(),
                DECKS[(i / 2 + 1 + i / 24) % DECKS.len()].to_owned(),
            ],
        })
        .collect();
    scored(player, opponent, games)
}

fn scored(player: &str, opponent: &str, games: Vec<Game>) -> Match {
    Match {
        source: format!("{player} v {opponent}"),
        kind: "eval4",
        player: player.to_owned(),
        opponent: opponent.to_owned(),
        format: "1v1".to_owned(),
        deals: games.iter().map(|game| game.deal + 1).max().unwrap_or(0),
        games,
        caps: 0,
        input: json!({}),
        note: None,
    }
}

/// Games with fixed scores, each its own deal.
fn results(player: &str, opponent: &str, scores: &[f64]) -> Match {
    let games = scores
        .iter()
        .enumerate()
        .map(|(deal, &score)| Game {
            score,
            deal,
            decks: vec![
                DECKS[deal % 12].to_owned(),
                DECKS[(deal + 5) % 12].to_owned(),
            ],
        })
        .collect();
    scored(player, opponent, games)
}

fn anchors(pairs: &[(&str, f64)]) -> BTreeMap<String, f64> {
    pairs
        .iter()
        .map(|&(name, rating)| (name.to_owned(), rating))
        .collect()
}

fn unfair(names: &[&str]) -> BTreeSet<String> {
    names.iter().map(|&name| name.to_owned()).collect()
}

fn player<'a>(report: &'a Value, format: &str, name: &str) -> &'a Value {
    report["formats"][format]["players"]
        .as_array()
        .unwrap()
        .iter()
        .find(|player| player["player"] == name)
        .unwrap_or_else(|| panic!("{name} not in {format}"))
}

fn rating(report: &Value, name: &str) -> Option<f64> {
    player(report, "1v1", name)["rating"].as_f64()
}

#[test]
fn synthetic_strengths_are_recovered() {
    let truth = [
        ("weak", 200.0),
        ("mid", 450.0),
        ("good", 800.0),
        ("best", 1_000.0),
    ];
    let true_of = |name: &str| match name {
        "random" => 0.0,
        "sharp" => 600.0,
        _ => truth.iter().find(|(n, _)| *n == name).unwrap().1,
    };
    let mut state = 7;
    let mut matches = Vec::new();
    for (player, opponent) in [
        ("weak", "random"),
        ("weak", "sharp"),
        ("mid", "sharp"),
        ("mid", "weak"),
        ("good", "sharp"),
        ("good", "mid"),
        ("best", "good"),
        ("best", "sharp"),
    ] {
        let difference = true_of(player) - true_of(opponent);
        matches.push(played(player, opponent, difference, 3_000, &mut state));
    }
    let report = rate_matches(
        &anchors(&[("random", 0.0), ("sharp", 600.0)]),
        &BTreeSet::new(),
        matches,
        &options(1),
    )
    .unwrap();
    for (name, value) in truth {
        let fitted = rating(&report, name).unwrap();
        assert!(
            (fitted - value).abs() < 30.0,
            "{name}: fitted {fitted}, true {value}"
        );
        for kind in ["by_deal", "by_deck"] {
            let ends = &player(&report, "1v1", name)[kind]["interval"];
            let (low, high) = (ends[0].as_f64().unwrap(), ends[1].as_f64().unwrap());
            assert!(
                low < fitted && fitted < high,
                "{name} {kind}: {low} {fitted} {high}"
            );
            assert!(high - low < 150.0, "{name} {kind}: [{low}, {high}]");
        }
    }
    // The fit check holds where the model generated the games.
    for pair in report["formats"]["1v1"]["pairs"].as_array().unwrap() {
        assert_eq!(pair["misfit"], false, "{pair}");
    }
}

#[test]
fn anchors_stay_fixed_even_against_their_own_games() {
    let even: Vec<f64> = (0..400).map(|i| f64::from(i % 2)).collect();
    let three_quarters: Vec<f64> = (0..400).map(|i| f64::from(u8::from(i % 4 != 0))).collect();
    let report = rate_matches(
        &anchors(&[("random", 0.0), ("sharp", 1_000.0)]),
        &BTreeSet::new(),
        vec![
            results("random", "sharp", &even),
            results("net", "sharp", &three_quarters),
        ],
        &options(3),
    )
    .unwrap();
    assert_eq!(rating(&report, "random"), Some(0.0));
    assert_eq!(rating(&report, "sharp"), Some(1_000.0));
    for name in ["random", "sharp"] {
        let anchor = player(&report, "1v1", name);
        assert_eq!(anchor["status"], "anchor");
        assert!(anchor["by_deal"].is_null() && anchor["by_deck"].is_null());
    }
    // 75 % is 191 Elo above the anchor it was scored against.
    let net = rating(&report, "net").unwrap();
    assert!((net - 1_190.85).abs() < 0.1, "{net}");
    let pair = report["formats"]["1v1"]["pairs"]
        .as_array()
        .unwrap()
        .iter()
        .find(|pair| pair["players"] == json!(["sharp", "random"]))
        .unwrap();
    assert_eq!(pair["misfit"], true, "an even score 1000 apart: {pair}");
    assert_eq!(pair["observed_gap"].as_f64(), Some(0.0));
}

#[test]
fn draws_count_one_half() {
    let draws = vec![0.5; 100];
    let mixed: Vec<f64> = (0..100)
        .map(|i| match i % 10 {
            0..=2 => 1.0,
            3..=6 => 0.5,
            _ => 0.0,
        })
        .collect();
    let report = rate_matches(
        &anchors(&[("sharp", 1_000.0)]),
        &BTreeSet::new(),
        vec![
            results("drawer", "sharp", &draws),
            results("mixed", "sharp", &mixed),
        ],
        &options(5),
    )
    .unwrap();
    let drawer = rating(&report, "drawer").unwrap();
    let mixed = rating(&report, "mixed").unwrap();
    assert!((drawer - 1_000.0).abs() < 1e-6, "{drawer}");
    assert!((mixed - 1_000.0).abs() < 1e-6, "{mixed}");
    let input = &report["formats"]["1v1"]["inputs"][1];
    assert_eq!(
        (
            input["won"].clone(),
            input["drawn_or_capped"].clone(),
            input["lost"].clone()
        ),
        (json!(30), json!(40), json!(30))
    );
}

#[test]
fn formats_are_never_mixed() {
    let strong: Vec<f64> = (0..400).map(|i| f64::from(u8::from(i % 4 != 0))).collect();
    let weak: Vec<f64> = (0..400).map(|i| f64::from(u8::from(i % 4 == 0))).collect();
    let mut teams = results("net", "sharp", &weak);
    teams.format = "2v2".to_owned();
    let report = rate_matches(
        &anchors(&[("sharp", 1_000.0)]),
        &BTreeSet::new(),
        vec![results("net", "sharp", &strong), teams],
        &options(9),
    )
    .unwrap();
    let one = player(&report, "1v1", "net")["rating"].as_f64().unwrap();
    let two = player(&report, "2v2", "net")["rating"].as_f64().unwrap();
    assert!(
        (one - 1_190.85).abs() < 0.1 && (two - 809.15).abs() < 0.1,
        "{one} {two}"
    );
    for format in ["1v1", "2v2"] {
        assert_eq!(player(&report, format, "net")["games"], 400.0);
        assert_eq!(
            report["formats"][format]["inputs"]
                .as_array()
                .unwrap()
                .len(),
            1
        );
    }
}

#[test]
fn a_player_without_a_path_to_an_anchor_is_refused() {
    let half: Vec<f64> = (0..50).map(|i| f64::from(i % 2)).collect();
    let report = rate_matches(
        &anchors(&[("sharp", 1_000.0)]),
        &BTreeSet::new(),
        vec![
            results("net", "sharp", &half),
            results("island", "castaway", &half),
        ],
        &options(11),
    )
    .unwrap();
    assert!(rating(&report, "net").is_some());
    for name in ["island", "castaway"] {
        let refused = player(&report, "1v1", name);
        assert_eq!(refused["status"], "refused");
        assert!(refused["rating"].is_null());
        assert!(
            refused["flags"]
                .as_array()
                .unwrap()
                .iter()
                .any(|flag| flag.as_str().unwrap().starts_with("refused: no path")),
            "{refused}"
        );
    }
}

#[test]
fn the_bootstrap_repeats_with_its_seed() {
    let make = || {
        let mut state = 21;
        vec![
            played("net", "sharp", 50.0, 600, &mut state),
            played("llm", "net", -100.0, 200, &mut state),
        ]
    };
    let rate = |seed| {
        rate_matches(
            &anchors(&[("sharp", 1_000.0)]),
            &BTreeSet::new(),
            make(),
            &options(seed),
        )
        .unwrap()
    };
    assert_eq!(rate(4), rate(4));
    let (first, other) = (rate(4), rate(5));
    assert_eq!(rating(&first, "llm"), rating(&other, "llm"));
    assert_ne!(
        player(&first, "1v1", "llm")["by_deal"],
        player(&other, "1v1", "llm")["by_deal"]
    );
}

#[test]
fn unfair_players_never_move_or_link_fair_ratings() {
    let fair = || {
        let mut state = 31;
        vec![played("net", "sharp", 80.0, 800, &mut state)]
    };
    let alone = rate_matches(
        &anchors(&[("sharp", 1_000.0)]),
        &unfair(&["seer"]),
        fair(),
        &options(2),
    )
    .unwrap();
    let mut state = 32;
    let mut with_seer = fair();
    with_seer.extend([
        played("seer", "sharp", 300.0, 400, &mut state),
        played("seer", "net", 220.0, 400, &mut state),
        played("pupil", "seer", -150.0, 400, &mut state),
    ]);
    let report = rate_matches(
        &anchors(&[("sharp", 1_000.0)]),
        &unfair(&["seer"]),
        with_seer,
        &options(2),
    )
    .unwrap();
    assert_eq!(rating(&report, "net"), rating(&alone, "net"));
    let seer = player(&report, "1v1", "seer");
    assert_eq!(
        (seer["status"].clone(), seer["unfair"].clone()),
        (json!("rated"), json!(true))
    );
    assert_eq!(seer["link_games"], 800.0, "both fair opponents link it");
    // Linked only through the unfair player: refused.
    assert_eq!(player(&report, "1v1", "pupil")["status"], "refused");
    let refused = Spec {
        matches: vec![],
        anchors: anchors(&[("seer", 0.0)]),
        unfair: unfair(&["seer"]),
    };
    assert!(refused.check().is_err());
}

#[test]
fn a_player_who_never_lost_sits_above_every_rating() {
    let report = rate_matches(
        &anchors(&[("sharp", 1_000.0)]),
        &BTreeSet::new(),
        vec![
            results("perfect", "sharp", &[1.0; 40]),
            results("victim", "perfect", &[0.0, 0.5, 0.0, 0.5]),
            results("hopeless", "sharp", &[0.0; 40]),
        ],
        &options(13),
    )
    .unwrap();
    assert_eq!(player(&report, "1v1", "perfect")["status"], "above");
    assert_eq!(player(&report, "1v1", "hopeless")["status"], "below");
    // Only a player above every rating ever scored against it: nothing bounds
    // it from above either.
    assert_eq!(player(&report, "1v1", "victim")["status"], "above");
    assert!(rating(&report, "perfect").is_none());
}

#[test]
fn thin_links_are_flagged_with_their_games() {
    let mut state = 41;
    let report = rate_matches(
        &anchors(&[("sharp", 1_000.0)]),
        &BTreeSet::new(),
        vec![
            played("net", "sharp", 0.0, 1_000, &mut state),
            played("llm", "net", 0.0, 50, &mut state),
            played("llm", "sharp", 0.0, 30, &mut state),
        ],
        &options(17),
    )
    .unwrap();
    let llm = player(&report, "1v1", "llm");
    assert_eq!(llm["link_games"], 80.0);
    assert!(
        llm["flags"]
            .as_array()
            .unwrap()
            .iter()
            .any(|flag| flag == "thin: 80 games link it to an anchor"),
        "{llm}"
    );
    let net = player(&report, "1v1", "net");
    assert_eq!(net["link_games"], 1_030.0);
    assert!(net["flags"].as_array().unwrap().is_empty(), "{net}");
}

fn write(path: &Path, value: &Value) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, serde_json::to_vec(value).unwrap()).unwrap();
}

/// An `eval4` row of `table`, the net in `seat`.
fn row(table: u64, seat: u64, utility: f64, capped: bool) -> Value {
    json!({"deal": table * 2 + seat, "table": table, "seed": 100 + table, "focal_seat": seat,
        "decks": [format!("eval/d{table}"), "eval/x"], "net_utility": utility,
        "net_capped": capped})
}

#[test]
fn summaries_and_ladder_roots_are_read_as_games() {
    let dir = tempfile::tempdir().unwrap();
    let base = dir.path();
    write(
        &base.join("eval.json"),
        &json!({"seats": 2, "teams": [], "against": "sharp", "model": "m", "policy_mode": "raw",
            "dev_arms": {"choice_guard": "tactical", "tactics_rules": crate::measured::RULE_GROUPS},
            "deal_details": [row(0, 0, 1.0, false), row(0, 1, 0.5, true), row(1, 0, 0.0, false),
                row(1, 1, 1.0, false)]}),
    );
    write(
        &base.join("teams.json"),
        &json!({"seats": 4, "teams": [1, 2, 1, 2], "against": "sharp",
            "deal_details": [row(0, 0, 1.0, false)]}),
    );
    let root = base.join("ladder");
    let game = |number: u64, won: Option<bool>| {
        json!({"game": number, "profile": "deepseek-low", "mode": "tactical", "won": won,
            "drawn": false, "net_deck": "eval/a", "llm_deck": "eval/b"})
    };
    write(
        &root.join("ladder.json"),
        &json!({"games": [game(0, Some(true)), game(1, Some(false)), game(2, None),
            game(3, Some(true))]}),
    );
    let transcript = |number: u64, house: u64| {
        let path = root.join(format!("game-{number:04}/transcripts/room-seat1.jsonl"));
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        let last = json!({"summary": {"answered": {"mind": 40, "house": house}}});
        std::fs::write(path, format!("{{}}\n{last}\n")).unwrap();
    };
    transcript(0, 0);
    transcript(1, 1);
    // The house answered a fifth of game 3: not the model's game.
    transcript(3, 10);
    write(
        &base.join("spec.json"),
        &json!({"matches": [
            {"file": "eval.json", "player": "net+bundle", "opponent": "sharp"},
            {"ladder": "ladder", "player": "net+bundle", "opponent": "deepseek-low"}],
            "anchors": {"sharp": 1000}}),
    );
    let spec = Spec::read(&base.join("spec.json")).unwrap();
    let report = rate(&spec, &options(23)).unwrap();
    let inputs = report["formats"]["1v1"]["inputs"].as_array().unwrap();
    assert_eq!(
        (
            inputs[0]["games"].clone(),
            inputs[0]["deals"].clone(),
            inputs[0]["capped"].clone()
        ),
        (json!(4), json!(2), json!(1))
    );
    assert_eq!(inputs[0]["score"], 2.5);
    assert_eq!(
        inputs[0]["input"]["player"],
        "m raw, guard tactical, rules bundle"
    );
    assert_eq!(inputs[0]["input"]["opponent"], "house sharp");
    assert_eq!(
        (inputs[1]["games"].clone(), inputs[1]["won"].clone()),
        (json!(2), json!(1))
    );
    assert_eq!(inputs[1]["input"]["house_finished"], 1);
    assert_eq!(inputs[1]["input"]["lost_before_play"], 1);
    assert_eq!(report["formats"]["1v1"]["decks"], 5);
    let markdown = markdown(&report);
    assert!(
        markdown.contains("## 1v1") && markdown.contains("| net+bundle |"),
        "{markdown}"
    );
    // A summary whose format differs from the spec's is refused.
    write(
        &base.join("wrong.json"),
        &json!({"matches": [{"file": "teams.json", "player": "net", "opponent": "sharp",
            "format": "1v1"}], "anchors": {"sharp": 1000}}),
    );
    let wrong = rate(&Spec::read(&base.join("wrong.json")).unwrap(), &options(1));
    assert!(format!("{:#}", wrong.unwrap_err()).contains("is 2v2, the spec says 1v1"));
    // The same input twice, under two spellings, is refused.
    write(
        &base.join("twice.json"),
        &json!({"matches": [{"file": "eval.json", "player": "a", "opponent": "sharp"},
            {"file": "./eval.json", "player": "b", "opponent": "sharp"}],
            "anchors": {"sharp": 1000}}),
    );
    assert!(Spec::read(&base.join("twice.json")).is_err());
}
