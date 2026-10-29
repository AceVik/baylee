use super::{floats, metrics, ratio};
use crate::files;
use anyhow::{Context, Result, ensure};
use serde_json::{Value, json};
use std::{
    collections::{BTreeMap, BTreeSet},
    fmt::Write,
    path::{Path, PathBuf},
};
const BANDS: [&str; 5] = [
    "turns 1-3",
    "turns 4-6",
    "turns 7-9",
    "turns 10-14",
    "turns 15+",
];
fn ints(v: &Value) -> Result<Vec<i64>> {
    v.as_array()
        .context("value row vector")?
        .iter()
        .map(|n| n.as_i64().context("integer value row"))
        .collect()
}
pub(super) fn plan(input: &Value) -> Result<Value> {
    let rows = &input["rows"];
    let games = ints(&rows["game"])?;
    let seats = ints(&rows["seat"])?;
    let turns = ints(&rows["turn"])?;
    let outcomes = ints(&rows["result"])?;
    ensure!(
        [seats.len(), turns.len(), outcomes.len()]
            .iter()
            .all(|n| *n == games.len()),
        "value row lengths differ"
    );
    let steps = if rows["step"].is_null() {
        vec![0; games.len()]
    } else {
        ints(&rows["step"])?
    };
    ensure!(steps.len() == games.len(), "value step length differs");
    let use_deck = input["no_deck"] != true;
    let table = input["games"].as_array().context("value game table")?;
    let mut deck_names = BTreeSet::new();
    if use_deck {
        for game in table {
            for deck in game["decks"].as_array().context("game decks")? {
                deck_names.insert(deck.as_str().context("deck name")?.to_owned());
            }
        }
    }
    let deck_ids: BTreeMap<_, _> = deck_names
        .into_iter()
        .enumerate()
        .map(|(i, name)| (name, i))
        .collect();
    let mut fitted: BTreeMap<(usize, i64, usize), (f64, usize)> = BTreeMap::new();
    let mut eval = Vec::new();
    for i in 0..games.len() {
        ensure!(
            games[i] >= 0 && seats[i] >= 0 && [0, 1, 2].contains(&outcomes[i]),
            "invalid legacy value identity/outcome"
        );
        if steps[i] != 0 {
            continue;
        }
        let band = match turns[i] {
            ..=3 => 0,
            4..=6 => 1,
            7..=9 => 2,
            10..=14 => 3,
            _ => 4,
        };
        let deck = if use_deck {
            let game = table
                .get(usize::try_from(games[i])?)
                .context("value game index")?;
            let name = game["decks"]
                .get(usize::try_from(seats[i])?)
                .and_then(Value::as_str)
                .context("seat deck index")?;
            *deck_ids.get(name).context("unknown deck")?
        } else {
            0
        };
        let key = (deck, seats[i], band);
        let truth = f64::from(i32::try_from(outcomes[i])?) / 2.0;
        if games[i] % 2 == 0 {
            let entry = fitted.entry(key).or_default();
            entry.0 += truth;
            entry.1 += 1;
        } else {
            eval.push((i, key, truth));
        }
    }
    ensure!(
        !eval.is_empty(),
        "no held-out first decisions in value dataset"
    );
    let indices: Vec<_> = eval.iter().map(|v| v.0).collect();
    let truth: Vec<_> = eval.iter().map(|v| v.2).collect();
    let bands: Vec<_> = eval.iter().map(|v| v.1.2).collect();
    let baseline: Vec<_> = eval
        .iter()
        .map(|v| fitted.get(&v.1).map_or(0.5, |(sum, n)| sum / ratio(*n)))
        .collect();
    let game_count = eval
        .iter()
        .map(|v| games[v.0])
        .collect::<BTreeSet<_>>()
        .len();
    Ok(
        json!({"indices":indices,"truth":truth,"bands":bands,"baseline":baseline,"games":game_count,"baseline_description":if use_deck{"seat + turn band + deck"}else{"seat + turn band"}}),
    )
}
fn number(value: &Value, key: &str) -> String {
    value[key]
        .as_f64()
        .map_or_else(|| "n/a".into(), |v| format!("{v:.4}"))
}
fn expanded(path: &str) -> Result<PathBuf> {
    if let Some(rest) = path.strip_prefix("~/") {
        Ok(PathBuf::from(std::env::var_os("HOME").context("home path")?).join(rest))
    } else {
        Ok(PathBuf::from(path))
    }
}
pub(super) fn report(input: &Value) -> Result<Value> {
    let plan = &input["plan"];
    let prediction = floats(&input["prediction"])?;
    let truth = floats(&plan["truth"])?;
    let baseline = floats(&plan["baseline"])?;
    let bands = ints(&plan["bands"])?;
    ensure!(
        [truth.len(), baseline.len(), bands.len()]
            .iter()
            .all(|n| *n == prediction.len()),
        "value predictions differ from held-out plan"
    );
    let mut rows = Vec::new();
    for (band, name) in BANDS.iter().enumerate() {
        let indices: Vec<_> = bands
            .iter()
            .enumerate()
            .filter(|(_, v)| **v == i64::try_from(band).expect("five bands"))
            .map(|(i, _)| i)
            .collect();
        if indices.len() < 100 {
            continue;
        }
        let p: Vec<_> = indices.iter().map(|&i| prediction[i]).collect();
        let y: Vec<_> = indices.iter().map(|&i| truth[i]).collect();
        let b: Vec<_> = indices.iter().map(|&i| baseline[i]).collect();
        let mb = metrics(&b, &y)?;
        let mn = metrics(&p, &y)?;
        rows.push(json!({"band":name,"n":indices.len(),"baseline_brier":mb["brier"],"baseline_acc":mb["accuracy"],"net_brier":mn["brier"],"net_acc":mn["accuracy"],"net_ece":mn["ece"],"net_mean":p.iter().sum::<f64>()/ratio(p.len()),"won":y.iter().sum::<f64>()/ratio(y.len())}));
    }
    let mb = metrics(&baseline, &truth)?;
    let mn = metrics(&prediction, &truth)?;
    let report = json!({"model":input["model"],"data":input["data"],"aggregation":"native f64","baseline":plan["baseline_description"],"bands":rows,"all":{"n":prediction.len(),"baseline_brier":mb["brier"],"net_brier":mn["brier"],"net_ece":mn["ece"],"net_acc":mn["accuracy"]}});
    let mut text = format!(
        "baseline: {}; {} decisions of {} games\nband         n baseline Brier net Brier net ECE\n",
        plan["baseline_description"]
            .as_str()
            .context("baseline description")?,
        prediction.len(),
        plan["games"]
    );
    for row in &rows {
        writeln!(
            text,
            "{} {} {} {} {}",
            row["band"].as_str().context("band")?,
            row["n"],
            number(row, "baseline_brier"),
            number(row, "net_brier"),
            number(row, "net_ece")
        )?;
    }
    writeln!(
        text,
        "all {} {} {} {}",
        prediction.len(),
        number(&report["all"], "baseline_brier"),
        number(&report["all"], "net_brier"),
        number(&report["all"], "net_ece")
    )?;
    let data = Path::new(input["data"].as_str().context("data path")?);
    let name = data.file_name().context("dataset name")?.to_string_lossy();
    let output =
        expanded(input["model"].as_str().context("model path")?)?.join(format!("eval-{name}.json"));
    files::atomic_json(&output, &report)?;
    Ok(json!({"report":report,"text":text,"out":output}))
}
