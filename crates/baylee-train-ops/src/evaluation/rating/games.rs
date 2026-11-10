//! The inputs of a rating, read as games between two named players: the deals
//! of an `eval4` summary, the clean games of a `ladder4` root.
use super::spec::MatchSpec;
use crate::evaluation::games::game_rows;
use crate::evaluation::ladder::{clean_score, ladder_table};
use crate::evaluation::{case_key, flag, number, read_json};
use crate::measured::RULE_GROUPS;
use anyhow::{Context, Result, bail, ensure};
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

/// One game, from the player's side.
#[derive(Clone, Debug)]
pub(super) struct Game {
    /// 1 a win, ½ a draw or a deal stopped at its answer cap (as `eval4`'s
    /// `net_utility` counts one), 0 a loss; the mean where seats of one team
    /// replayed the same game.
    pub score: f64,
    /// The deal it belongs to within its match: an `eval4` table with every
    /// seat rotation of it, or a ladder's deck pair with both seatings. The
    /// bootstrap by deal draws these.
    pub deal: usize,
    /// Every seat's deck. The bootstrap by deck draws these.
    pub decks: Vec<String>,
    /// An `eval4` game's identity across inputs, `seed/table/side/decks`: two
    /// inputs of one pair holding the same are one measurement twice. The
    /// decks are part of it because two deck pools run from one first seed
    /// share seeds and tables but are different games.
    pub key: Option<String>,
}

/// One input: the games two named players played in it.
#[derive(Clone, Debug)]
pub(super) struct Match {
    /// The input's path.
    pub source: String,
    /// `eval4` or `ladder4`.
    pub kind: &'static str,
    pub player: String,
    pub opponent: String,
    /// `1v1`, `2v2`, ... or `ffaN`: which table the games go to.
    pub format: String,
    pub games: Vec<Game>,
    /// Distinct deals among `games`.
    pub deals: usize,
    /// Rows the input holds: an `eval4` deal per focal seat, a ladder game.
    pub rows: usize,
    /// Games stopped at their answer cap, counted one half.
    pub caps: usize,
    /// What the input itself says of its sides, to check the spec's names by.
    pub input: Value,
    /// What has to hold for it to be a game of these two players.
    pub assumptions: Vec<String>,
    /// Which `first_of` candidate was read, when the entry has them.
    pub chosen: Value,
    pub note: Option<String>,
}

/// What an input holds before the spec names its sides.
struct Read {
    format: String,
    games: Vec<Game>,
    rows: usize,
    caps: usize,
    input: Value,
}

/// The input an entry reads: its path, whether it is a ladder root, what it
/// assumes beside the entry, and which `first_of` candidate it is.
fn chosen(entry: &MatchSpec) -> Result<(PathBuf, bool, Vec<String>, Value)> {
    Ok(match (&entry.file, &entry.ladder, &entry.first_of) {
        (Some(file), None, None) => (file.clone(), false, Vec::new(), Value::Null),
        (None, Some(root), None) => (root.clone(), true, Vec::new(), Value::Null),
        (None, None, Some(candidates)) => {
            let (index, candidate) = candidates
                .iter()
                .enumerate()
                .find(|(_, candidate)| candidate.file.exists())
                .context("no `first_of` candidate exists")?;
            let skipped: Vec<_> = candidates[..index].iter().map(|c| &c.file).collect();
            (
                candidate.file.clone(),
                false,
                candidate.assumption.iter().cloned().collect(),
                json!({"candidate": index + 1, "of": candidates.len(), "missing": skipped}),
            )
        }
        _ => bail!("a match names one input, `file`, `ladder` or `first_of`"),
    })
}

/// Reads the input a spec entry names.
///
/// # Errors
/// Rejects unreadable inputs, inputs without games and a format other than the
/// entry's `format`.
pub(super) fn read(entry: &MatchSpec) -> Result<Match> {
    let (source, is_ladder, extra, chosen) = chosen(entry)?;
    let read = if is_ladder {
        ladder(&source, entry.profile.as_deref(), entry.mode.as_deref())
    } else {
        eval(&source)
    }
    .with_context(|| format!("input {}", source.display()))?;
    if let Some(format) = &entry.format {
        ensure!(
            *format == read.format,
            "{} is {}, the spec says {format}",
            source.display(),
            read.format
        );
    }
    let deals = read
        .games
        .iter()
        .map(|game| game.deal + 1)
        .max()
        .unwrap_or(0);
    Ok(Match {
        source: source.display().to_string(),
        kind: if is_ladder { "ladder4" } else { "eval4" },
        player: entry.player.clone(),
        opponent: entry.opponent.clone(),
        format: read.format,
        games: read.games,
        deals,
        rows: read.rows,
        caps: read.caps,
        input: read.input,
        assumptions: entry.assumption.iter().cloned().chain(extra).collect(),
        chosen,
        note: entry.note.clone(),
    })
}

/// The format of an `eval4` table: `1v1` for two seats, `AvB` for teams by
/// their sizes, `ffaN` for N seats without teams.
fn format_of(summary: &Value) -> Result<String> {
    let seats = summary["seats"].as_u64().context("summary without seats")?;
    let teams = summary["teams"].as_array().map_or(&[][..], Vec::as_slice);
    if teams.is_empty() {
        return Ok(if seats == 2 {
            "1v1".to_owned()
        } else {
            format!("ffa{seats}")
        });
    }
    ensure!(
        teams.len() as u64 == seats,
        "{} teams for {seats} seats",
        teams.len()
    );
    let mut sizes = BTreeMap::<u64, usize>::new();
    for team in teams {
        *sizes
            .entry(team.as_u64().context("a seat without a team")?)
            .or_default() += 1;
    }
    ensure!(sizes.len() >= 2, "every seat on one team");
    let mut sizes: Vec<usize> = sizes.into_values().collect();
    sizes.sort_unstable_by(|a, b| b.cmp(a));
    Ok(sizes
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join("v"))
}

/// The rule groups as a reader takes them in: `bundle` for the measured set.
fn groups(rules: &str) -> &str {
    if rules == RULE_GROUPS {
        "bundle"
    } else {
        rules
    }
}

/// What an `eval4` summary says of its two sides, with the label it gives the
/// house (`against`: `sharp` before the frozen `sharp-2026-11` landed, the frozen
/// name after), so a spec mapping both labels to one player is checked by it.
fn describe(summary: &Value) -> Value {
    let text = |value: &Value| value.as_str().unwrap_or("").to_owned();
    let player = if summary["random_policy"] == true {
        "random answers".to_owned()
    } else {
        let mut words = vec![format!(
            "{} {}",
            text(&summary["model"]),
            text(&summary["policy_mode"])
        )];
        let guard = text(&summary["dev_arms"]["choice_guard"]);
        if !guard.is_empty() && guard != "off" {
            words.push(format!("guard {guard}"));
        }
        let rules = text(&summary["dev_arms"]["tactics_rules"]);
        if !rules.is_empty() {
            words.push(format!("rules {}", groups(&rules)));
        }
        if let Some(steps) = summary["thinking_steps"].as_u64() {
            words.push(format!("K={steps}"));
        }
        words.join(", ")
    };
    let opponent = match summary["opponent"]["model"].as_str() {
        Some(model) => format!("{model} {}", text(&summary["opponent"]["policy"])),
        None => format!("house {}", text(&summary["against"])),
    };
    json!({"player": player, "opponent": opponent, "against": summary["against"],
        "evaluator_revision": summary["evaluator_revision"], "first_seed": summary["first_seed"],
        "deck_split": summary["deck_split"], "precision": summary["precision"],
        "seats": summary["seats"], "teams": summary["teams"]})
}

/// Every game of an `eval4` summary, as `evaluation::games` defines one: a
/// table and the side the net held, the rows of a team game made one at
/// their mean. The games of one table share a deal.
fn eval(path: &Path) -> Result<Read> {
    let summary = read_json(path)?;
    let format = format_of(&summary)?;
    let rows = summary["deal_details"]
        .as_array()
        .context("summary without deal_details")?;
    ensure!(!rows.is_empty(), "summary without deals");
    let mut caps = 0;
    for row in rows {
        caps += usize::from(flag(row, "net_capped")?);
    }
    let mut tables = BTreeMap::new();
    let mut games = Vec::new();
    for game in game_rows(&summary)? {
        let score = number(&game, "net_utility")?;
        ensure!(
            (0.0..=1.0).contains(&score),
            "net_utility {score} outside [0, 1]"
        );
        let (table, seed, decks, _) = case_key(&game, false)?;
        let name = game["game"].as_str().context("a game without its key")?;
        let next = tables.len();
        let deal = *tables.entry((table, seed, decks.clone())).or_insert(next);
        let key = format!("{seed}/{name}/{}", decks.join(","));
        games.push(Game {
            score,
            deal,
            decks,
            key: Some(key),
        });
    }
    Ok(Read {
        format,
        games,
        rows: rows.len(),
        caps,
        input: describe(&summary),
    })
}

/// The clean games of a `ladder4` root, as `phase_a4 ladder` counts them, of
/// one profile and mode: a root of several needs `profile` and `mode`.
fn ladder(root: &Path, profile: Option<&str>, mode: Option<&str>) -> Result<Read> {
    let table = ladder_table(&[root.to_path_buf()])?;
    let text = |game: &Value, key: &str| game[key].as_str().unwrap_or("").to_owned();
    let chosen: Vec<&Value> = table["games"]
        .as_array()
        .context("no games")?
        .iter()
        .filter(|game| profile.is_none_or(|profile| text(game, "profile") == profile))
        .filter(|game| mode.is_none_or(|mode| text(game, "mode") == mode))
        .collect();
    let arms: BTreeSet<(String, String)> = chosen
        .iter()
        .map(|game| (text(game, "profile"), text(game, "mode")))
        .collect();
    ensure!(
        arms.len() == 1,
        "games of {} profile and mode pairs {arms:?}: name one with `profile` and `mode`",
        arms.len()
    );
    let mut pairs = BTreeMap::new();
    let mut games = Vec::new();
    let (mut lost, mut house) = (0_usize, 0_usize);
    for game in &chosen {
        let Some(score) = clean_score(game) else {
            if game["played"] == true {
                house += 1;
            } else {
                lost += 1;
            }
            continue;
        };
        let decks: Vec<String> = ["net_deck", "llm_deck"]
            .iter()
            .filter_map(|key| game[*key].as_str().map(str::to_owned))
            .collect();
        let mut pair = decks.clone();
        pair.sort();
        let next = pairs.len();
        let deal = *pairs.entry(pair).or_insert(next);
        games.push(Game {
            score,
            deal,
            decks,
            key: None,
        });
    }
    ensure!(!games.is_empty(), "no clean game");
    let (profile, mode) = arms.into_iter().next().context("no arm")?;
    let mode = mode.replace(RULE_GROUPS, "bundle");
    Ok(Read {
        format: "1v1".to_owned(),
        games,
        rows: chosen.len(),
        caps: 0,
        input: json!({"player": format!("net {mode}"), "opponent": profile,
            "scheduled": chosen.len(), "lost_before_play": lost, "house_finished": house}),
    })
}
