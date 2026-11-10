//! A rating's table as JSON, and as markdown read back from that JSON.
use super::fit::{Pair, Place, expected};
use super::games::Match;
use super::{Options, Roster, bootstrap};
use serde_json::{Value, json};
use std::fmt::Write as _;

/// A residual this many standard errors from the fit marks a pair that does
/// not fit.
const MISFIT_Z: f64 = 3.0;

/// What one format's table is made of.
pub(super) struct Table<'a> {
    pub roster: &'a Roster,
    pub matches: &'a [Match],
    /// Every pair's games, unweighted.
    pub observed: &'a [Pair],
    pub fitted: &'a [Place],
    pub links: &'a [Option<f64>],
    pub by_deal: &'a [Vec<f64>],
    pub by_deck: Option<&'a [Vec<f64>]>,
    pub decks: usize,
    pub without_decks: usize,
    pub options: &'a Options,
}

fn rating(place: Place) -> Option<f64> {
    match place {
        Place::Rated(rating) => Some(rating),
        _ => None,
    }
}

/// The player's flags: why it has no rating, and what its rating rests on.
fn flags(table: &Table<'_>, player: usize, deal: &Value, deck: &Value) -> Vec<String> {
    let roster = table.roster;
    let mut flags = Vec::new();
    let reference = if roster.unfair[player] {
        "the rated fair players"
    } else {
        "an anchor"
    };
    if let Some(value) = roster.held[player] {
        flags.push(format!("anchor, held at {value}"));
    }
    if roster.unfair[player] {
        flags.push("unfair: rated against the fair ratings held, never a link".to_owned());
    }
    match table.fitted[player] {
        Place::Rated(_) => {}
        Place::Above => flags.push(format!(
            "above every rating: never scored against on a path from {reference}"
        )),
        Place::Below => flags.push(format!(
            "below every rating: never scored on a path to {reference}"
        )),
        Place::Undetermined => flags.push(format!(
            "no rating: linked to {reference} only through players without one"
        )),
        Place::Unanchored => flags.push(format!("refused: no path of games to {reference}")),
    }
    if let Some(link) = table.links[player]
        && link < table.options.thin_link_games
        && matches!(table.fitted[player], Place::Rated(_))
    {
        flags.push(format!("thin: {link} games link it to {reference}"));
    }
    for (kind, spread) in [("deal", deal), ("deck", deck)] {
        let lost =
            spread["unrated"].as_u64().unwrap_or(0) + spread["unbounded"].as_u64().unwrap_or(0);
        if lost > 0 && roster.held[player].is_none() {
            flags.push(format!(
                "by {kind}: no finite rating in {lost} of {} resamples",
                spread["resamples"]
            ));
        }
    }
    flags
}

/// The table of one format as JSON.
pub(super) fn table(table: &Table<'_>) -> Value {
    let roster = table.roster;
    let mut players = Vec::new();
    for (player, name) in roster.names.iter().enumerate() {
        let mut opponents = Vec::new();
        let (mut games, mut score) = (0.0, 0.0);
        for pair in table.observed {
            let (other, own) = if pair.a == player {
                (pair.b, pair.score)
            } else if pair.b == player {
                (pair.a, pair.games - pair.score)
            } else {
                continue;
            };
            games += pair.games;
            score += own;
            opponents.push(json!({"opponent": roster.names[other], "games": pair.games,
                "score": own / pair.games}));
        }
        let games_of = |opponent: &Value| opponent["games"].as_f64().unwrap_or(0.0);
        opponents.sort_by(|a, b| games_of(b).total_cmp(&games_of(a)));
        let held = roster.held[player].is_some();
        let deal = if held {
            Value::Null
        } else {
            bootstrap::spread(&table.by_deal[player])
        };
        let deck = match table.by_deck {
            Some(draws) if !held => bootstrap::spread(&draws[player]),
            _ => Value::Null,
        };
        let status = match table.fitted[player] {
            _ if held => "anchor",
            Place::Rated(_) => "rated",
            Place::Above => "above",
            Place::Below => "below",
            Place::Undetermined => "undetermined",
            Place::Unanchored => "refused",
        };
        players.push(json!({"player": name, "status": status,
            "rating": rating(table.fitted[player]), "by_deal": deal, "by_deck": deck,
            "games": games, "score": if games > 0.0 { Some(score / games) } else { None },
            "opponents": opponents, "link_games": table.links[player],
            "unfair": roster.unfair[player],
            "flags": flags(table, player, &deal, &deck)}));
    }
    let order = |player: &Value| -> (u8, f64) {
        let rank = match player["status"].as_str() {
            Some("above") => 0,
            Some("rated" | "anchor") => 1,
            Some("below") => 2,
            Some("undetermined") => 3,
            _ => 4,
        };
        (rank, -player["rating"].as_f64().unwrap_or(0.0))
    };
    players.sort_by(|a, b| {
        let (a, b) = (order(a), order(b));
        a.0.cmp(&b.0).then(a.1.total_cmp(&b.1))
    });
    json!({"players": players, "pairs": fit_check(table), "inputs": inputs(table.matches),
        "decks": table.decks, "games_without_decks": table.without_decks})
}

/// Every pair's observed score next to the one the fit implies, the higher
/// rated player first.
fn fit_check(table: &Table<'_>) -> Vec<Value> {
    let names = &table.roster.names;
    table
        .observed
        .iter()
        .map(|pair| {
            let (ra, rb) = (rating(table.fitted[pair.a]), rating(table.fitted[pair.b]));
            let swap = matches!((ra, rb), (Some(ra), Some(rb)) if rb > ra);
            let (a, b, score, ra, rb) = if swap {
                (pair.b, pair.a, pair.games - pair.score, rb, ra)
            } else {
                (pair.a, pair.b, pair.score, ra, rb)
            };
            let observed = score / pair.games;
            let gap = ra.zip(rb).map(|(ra, rb)| ra - rb);
            let implied = gap.map(expected);
            let z = implied
                .filter(|p| *p > 0.0 && *p < 1.0)
                .map(|p| (observed - p) / (p * (1.0 - p) / pair.games).sqrt());
            let observed_gap = (observed > 0.0 && observed < 1.0)
                .then(|| 400.0 * (observed / (1.0 - observed)).log10());
            json!({"players": [names[a], names[b]], "games": pair.games, "observed": observed,
                "implied": implied, "z": z, "observed_gap": observed_gap, "fitted_gap": gap,
                "misfit": z.is_some_and(|z| z.abs() > MISFIT_Z)})
        })
        .collect()
}

/// The inputs, with what each says of its own sides.
fn inputs(matches: &[Match]) -> Vec<Value> {
    matches
        .iter()
        .map(|found| {
            let count =
                |test: fn(f64) -> bool| found.games.iter().filter(|game| test(game.score)).count();
            let mut decks: Vec<&String> = found.games.iter().flat_map(|g| &g.decks).collect();
            decks.sort_unstable();
            decks.dedup();
            json!({"source": found.source, "kind": found.kind, "player": found.player,
                "opponent": found.opponent, "games": found.games.len(), "deals": found.deals,
                "score": found.games.iter().map(|g| g.score).sum::<f64>(),
                "won": count(|s| s >= 1.0), "lost": count(|s| s <= 0.0),
                "drawn_or_capped": count(|s| s > 0.0 && s < 1.0), "capped": found.caps,
                "decks": decks.len(), "input": found.input, "note": found.note})
        })
        .collect()
}

fn whole(value: &Value) -> String {
    value
        .as_f64()
        .map_or_else(|| "open".to_owned(), |x| format!("{x:.0}"))
}

fn percent(value: &Value) -> String {
    value
        .as_f64()
        .map_or_else(String::new, |x| format!("{:.1} %", 100.0 * x))
}

fn interval(spread: &Value) -> String {
    if spread.is_null() {
        return String::new();
    }
    let ends = &spread["interval"];
    format!("[{}, {}]", whole(&ends[0]), whole(&ends[1]))
}

fn text(value: &Value) -> &str {
    value.as_str().unwrap_or("")
}

/// A number when there is one, nothing otherwise.
fn optional(value: &Value) -> String {
    if value.is_null() {
        String::new()
    } else {
        whole(value)
    }
}

/// The items of a JSON array, none for anything else.
fn items(value: &Value) -> impl Iterator<Item = &Value> {
    value.as_array().into_iter().flatten()
}

/// The report as markdown: per format the ratings, the fit check and the
/// inputs each rating rests on.
pub fn markdown(report: &Value) -> String {
    let mut out = String::from("# Rating ladder\n\n");
    let anchors: Vec<String> = report["anchors"]
        .as_object()
        .into_iter()
        .flatten()
        .map(|(name, value)| format!("{name} = {value}"))
        .collect();
    let _ = writeln!(
        out,
        "Bradley-Terry on the Elo scale: a player rated d above another scores 1 / (1 + 10^(-d/400)). \
         A draw, or a deal stopped at its answer cap, counts one half. Anchors held: {}. \
         95 % intervals from {} resamples (seed {}): **by deal**, each input's deals drawn again \
         (a deal keeps its seat rotations); **by deck**, the format's decks drawn again, a game \
         counting once per draw of each of its decks. **link** is the fewest games whose removal \
         would cut the player off the anchors; under {} it is flagged thin. Unfair players are \
         rated after the fair ones, against them held.\n",
        anchors.join(", "),
        report["resamples"],
        report["seed"],
        report["thin_link_games"]
    );
    for (format, table) in report["formats"].as_object().into_iter().flatten() {
        let _ = writeln!(out, "## {format}\n");
        players(&mut out, table);
        let _ = writeln!(
            out,
            "\n{} decks; {} games without decks.\n",
            table["decks"], table["games_without_decks"]
        );
        pairs(&mut out, table);
        inputs_table(&mut out, table);
    }
    out
}

/// The ratings, best first, with what each rests on.
fn players(out: &mut String, table: &Value) {
    out.push_str(
        "| player | rating | by deal | by deck | games | link | rests on (games, its score) | flags |\n\
         | --- | --: | --- | --- | --: | --: | --- | --- |\n",
    );
    for player in items(&table["players"]) {
        let rests: Vec<String> = items(&player["opponents"])
            .map(|o| {
                format!(
                    "{} {} ({})",
                    text(&o["opponent"]),
                    whole(&o["games"]),
                    percent(&o["score"])
                )
            })
            .collect();
        let rated = match text(&player["status"]) {
            "rated" | "anchor" => whole(&player["rating"]),
            other => other.to_owned(),
        };
        let flags: Vec<&str> = items(&player["flags"]).filter_map(Value::as_str).collect();
        let _ = writeln!(
            out,
            "| {} | {rated} | {} | {} | {} | {} | {} | {} |",
            text(&player["player"]),
            interval(&player["by_deal"]),
            interval(&player["by_deck"]),
            whole(&player["games"]),
            optional(&player["link_games"]),
            rests.join("; "),
            flags.join("; ")
        );
    }
}

/// Every pair's observed score against the one the fit implies.
fn pairs(out: &mut String, table: &Value) {
    let _ = writeln!(
        out,
        "### Fit check: observed against implied\n\n\
         The first player's score. z is the residual in binomial standard errors of the games; \
         over {MISFIT_Z} the pair does not fit. The observed gap is what the pair alone says.\n\n\
         | pair | games | observed | implied | z | observed gap | fitted gap |\n\
         | --- | --: | --: | --: | --: | --: | --: |"
    );
    for pair in items(&table["pairs"]) {
        let z = pair["z"]
            .as_f64()
            .map_or_else(String::new, |z| format!("{z:+.1}"));
        let misfit = if pair["misfit"] == true {
            " **misfit**"
        } else {
            ""
        };
        let _ = writeln!(
            out,
            "| {} v {} | {} | {} | {} | {z}{misfit} | {} | {} |",
            text(&pair["players"][0]),
            text(&pair["players"][1]),
            whole(&pair["games"]),
            percent(&pair["observed"]),
            percent(&pair["implied"]),
            optional(&pair["observed_gap"]),
            optional(&pair["fitted_gap"])
        );
    }
}

/// The inputs, and what each says of its own sides.
fn inputs_table(out: &mut String, table: &Value) {
    out.push_str(
        "\n### Inputs\n\n\
         | input | player | opponent | games | won-drawn-lost | caps | the input says |\n\
         | --- | --- | --- | --: | --- | --: | --- |\n",
    );
    for input in items(&table["inputs"]) {
        let _ = writeln!(
            out,
            "| {} | {} | {} | {} | {}-{}-{} | {} | {} against {} |",
            text(&input["source"]),
            text(&input["player"]),
            text(&input["opponent"]),
            input["games"],
            input["won"],
            input["drawn_or_capped"],
            input["lost"],
            input["capped"],
            text(&input["input"]["player"]),
            text(&input["input"]["opponent"])
        );
    }
    out.push('\n');
}
