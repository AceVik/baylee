//! What round 0's out-of-scope disagreements are, read from the games.
//!
//! At every labelled decision of a bundle-distillation run
//! (`eval4 --distill-labels`) where the net's answer `N` differs from the
//! layer's `R` and `R`'s subtype is `mana`, `pay` or `activate`, the audit
//! says which kind of disagreement it is:
//!
//! - one side passes priority (or cancels the cast) and the other acts:
//!   `R` acts and `N` passes, or `N` acts and `R` passes;
//! - both act, the same kind of action (another source, say) or another kind.
//!
//! Where `R` acts and `N` passes, it follows the seat's next inputs (the
//! layer played `R`): whether a cast or an activation starts within
//! `--window` of them, whether it reaches the stack (its name on the stack at
//! the seat's next decision, no `CancelCast` first), and the subtype the cast
//! decision's own label has. It also says what `N` did at the `pass` rows
//! (`R` passes, `N` acts). The games are replayed (`baylee_gamehost`), so the
//! actions are the ones played and the names the ones the seat saw.
//!
//! ```text
//! scope_audit4 --run RUN [--run RUN ...] --out report.json
//! ```

// Looks cards up, so this program holds the pool (`baylee-cards-pool`).
use baylee_cards_pool as _;

use std::collections::BTreeMap;
use std::fs::File;
use std::io::{BufRead as _, BufReader, Read as _, Seek as _, SeekFrom};
use std::path::PathBuf;
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};

use anyhow::{Context as _, ensure};
use baylee_core::ids::PlayerId;
use baylee_engine::choice::PlayerAction;
use baylee_gamehost::Session;
use baylee_gamehost::record::{Line, Source};
use baylee_train::convert4::distill_labels::{
    DistillStep4, DistillSubtype4, is_mana_ability_action as is_tap,
};
use baylee_view::PlayerView;
use clap::Parser;
use flate2::read::GzDecoder;
use serde::Serialize;
use serde_json::{Value, json};

#[derive(Parser)]
#[command(about = "Round 0's out-of-scope disagreements, read from the games")]
struct Args {
    /// A bundle-distillation run directory (`run.json`, `games.jsonl`, `records/`).
    #[arg(long, required = true)]
    run: Vec<PathBuf>,
    /// The seat's inputs after a decision a chain is followed through.
    #[arg(long, default_value_t = 6)]
    window: usize,
    /// Worker threads.
    #[arg(long, default_value_t = 8)]
    threads: usize,
    /// Examples kept per class.
    #[arg(long, default_value_t = 8)]
    examples: usize,
    /// The report, as JSON.
    #[arg(long)]
    out: PathBuf,
}

/// What a label says, as the audit needs it.
#[derive(Clone, Debug, Default)]
struct Label {
    subtype: Option<DistillSubtype4>,
    n_ne_r: bool,
    rules: Option<PlayerAction>,
    net: Option<PlayerAction>,
    rules_name: Option<String>,
    net_name: Option<String>,
    /// Whether `R`'s and `N`'s actions are mana abilities ([`is_tap`]).
    rules_tap: bool,
    net_tap: bool,
}

/// One input of a recorded seat.
#[derive(Clone, Debug, Default)]
struct Move {
    /// What the seat played.
    action: Option<PlayerAction>,
    /// Whether it is a mana ability ([`is_tap`]).
    tap: bool,
    /// What the action names (a card in hand, a source).
    name: Option<String>,
    /// Whether the seat owed a cost at the decision.
    owed: bool,
    /// Whether it was the seat's own turn.
    my_turn: bool,
    /// The names on the stack at the decision.
    stack: Vec<String>,
    /// The decision's label, when the policy answered it.
    label: Option<Label>,
}

/// Whether an answer passes (priority, or the cast it cancels) or acts.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Side {
    Passes,
    Acts,
    Unknown,
}

fn side(action: Option<&PlayerAction>) -> Side {
    match action {
        None => Side::Unknown,
        Some(PlayerAction::PassPriority | PlayerAction::CancelCast) => Side::Passes,
        Some(_) => Side::Acts,
    }
}

/// An action's variant, as JSON names it.
fn variant(action: &PlayerAction) -> String {
    match serde_json::to_value(action) {
        Ok(Value::String(name)) => name,
        Ok(Value::Object(map)) => map.keys().next().cloned().unwrap_or_default(),
        _ => "?".to_owned(),
    }
}

/// An action's kind for comparing answers: every mana ability is a `Tap`,
/// whether taken as `ActivateManaAbility` or through `ActivateAbility`.
fn kind(action: &PlayerAction, tap: bool) -> String {
    if tap {
        "Tap".to_owned()
    } else {
        variant(action)
    }
}

/// The kind of an out-of-scope disagreement.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Class {
    RActsNPasses,
    NActsRPasses,
    BothActSameKind,
    BothActOtherKind,
    Undecoded,
}

fn classify(label: &Label) -> Class {
    match (side(label.rules.as_ref()), side(label.net.as_ref())) {
        (Side::Acts, Side::Passes) => Class::RActsNPasses,
        (Side::Passes, Side::Acts) => Class::NActsRPasses,
        (Side::Acts, Side::Acts) => {
            let r = label.rules.as_ref().map(|a| kind(a, label.rules_tap));
            let n = label.net.as_ref().map(|a| kind(a, label.net_tap));
            if r == n {
                Class::BothActSameKind
            } else {
                Class::BothActOtherKind
            }
        }
        _ => Class::Undecoded,
    }
}

/// What the seat's chain did after a decision where `R` acted.
#[derive(Clone, Debug, PartialEq, Eq)]
enum Chain {
    /// A cast or an activation started at input `start` and reached the
    /// stack (its name there at a later decision of the seat).
    Completed { start: usize, cast: bool },
    /// It started and was cancelled.
    Cancelled { start: usize, cast: bool },
    /// It started; the game's inputs ran out before it was seen on the stack.
    Unfinished { start: usize, cast: bool },
    /// A payment in progress ended without a cancel.
    PaymentCompleted,
    /// A payment in progress was cancelled.
    PaymentCancelled,
    /// Nothing was cast or activated within the window.
    Nothing,
}

/// Follows the seat's inputs after `moves[at]`, where `R` acted.
fn chain(moves: &[Move], at: usize, window: usize) -> Chain {
    const LOOK: usize = 24;
    if moves[at].owed {
        for later in moves.iter().skip(at + 1).take(LOOK) {
            if matches!(later.action, Some(PlayerAction::CancelCast)) {
                return Chain::PaymentCancelled;
            }
            if !later.owed {
                return Chain::PaymentCompleted;
            }
        }
        return Chain::Nothing;
    }
    // A mana ability taken through `ActivateAbility` (a dual land's colour
    // choice, an Eldrazi Scion) is a tap of the chain, not its start.
    let starts = |m: &Move| {
        matches!(m.action, Some(PlayerAction::CastSpell { .. }))
            || (matches!(m.action, Some(PlayerAction::ActivateAbility { .. })) && !m.tap)
    };
    let start = if starts(&moves[at]) {
        Some(at)
    } else {
        (at + 1..moves.len().min(at + 1 + window)).find(|&j| starts(&moves[j]))
    };
    let Some(start) = start else {
        return Chain::Nothing;
    };
    let cast = matches!(moves[start].action, Some(PlayerAction::CastSpell { .. }));
    let name = moves[start].name.clone();
    for later in moves.iter().skip(start + 1).take(LOOK) {
        if matches!(later.action, Some(PlayerAction::CancelCast)) {
            return Chain::Cancelled { start, cast };
        }
        if name.as_ref().is_some_and(|n| later.stack.contains(n)) {
            return Chain::Completed { start, cast };
        }
    }
    Chain::Unfinished { start, cast }
}

fn bump(map: &mut BTreeMap<String, u64>, key: impl Into<String>) {
    *map.entry(key.into()).or_default() += 1;
}

fn merge(into: &mut BTreeMap<String, u64>, from: &BTreeMap<String, u64>) {
    for (key, n) in from {
        *into.entry(key.clone()).or_default() += n;
    }
}

/// What `R`'s chains did, over the `R`-acts rows of a subtype.
#[derive(Clone, Debug, Default, Serialize)]
struct ChainTally {
    cast_completed: u64,
    cast_cancelled: u64,
    cast_unfinished: u64,
    activation_completed: u64,
    activation_cancelled: u64,
    activation_unfinished: u64,
    payment_completed: u64,
    payment_cancelled: u64,
    nothing_in_window: u64,
    /// Inputs from the decision to the cast or activation (0: the decision itself).
    steps_to_start: BTreeMap<String, u64>,
    /// The subtype of the cast decision's own label (`unlabelled` for a forced one).
    cast_step_subtype: BTreeMap<String, u64>,
    /// Whether `N` and `R` differ at that cast decision.
    cast_step_n_ne_r: u64,
    cast_step_n_eq_r: u64,
    spells: BTreeMap<String, u64>,
}

impl ChainTally {
    /// Follows `R`'s chain after `moves[at]` and counts what it did.
    fn count(&mut self, moves: &[Move], at: usize, window: usize) -> Chain {
        let followed = chain(moves, at, window);
        let started = match &followed {
            Chain::Completed { start, cast } => {
                if *cast {
                    self.cast_completed += 1;
                } else {
                    self.activation_completed += 1;
                }
                Some((*start, *cast))
            }
            Chain::Cancelled { start, cast } => {
                if *cast {
                    self.cast_cancelled += 1;
                } else {
                    self.activation_cancelled += 1;
                }
                Some((*start, *cast))
            }
            Chain::Unfinished { start, cast } => {
                if *cast {
                    self.cast_unfinished += 1;
                } else {
                    self.activation_unfinished += 1;
                }
                Some((*start, *cast))
            }
            Chain::PaymentCompleted => {
                self.payment_completed += 1;
                None
            }
            Chain::PaymentCancelled => {
                self.payment_cancelled += 1;
                None
            }
            Chain::Nothing => {
                self.nothing_in_window += 1;
                None
            }
        };
        if let Some((start, true)) = started {
            let step = &moves[start];
            if let Some(name) = &step.name {
                bump(&mut self.spells, name.clone());
            }
            match &step.label {
                Some(l) => {
                    bump(
                        &mut self.cast_step_subtype,
                        l.subtype.map_or("none", DistillSubtype4::name),
                    );
                    if l.n_ne_r {
                        self.cast_step_n_ne_r += 1;
                    } else {
                        self.cast_step_n_eq_r += 1;
                    }
                }
                None => bump(&mut self.cast_step_subtype, "unlabelled"),
            }
        }
        if let Some((start, _)) = started {
            bump(&mut self.steps_to_start, (start - at).to_string());
        }
        followed
    }

    fn add(&mut self, other: &Self) {
        self.cast_completed += other.cast_completed;
        self.cast_cancelled += other.cast_cancelled;
        self.cast_unfinished += other.cast_unfinished;
        self.activation_completed += other.activation_completed;
        self.activation_cancelled += other.activation_cancelled;
        self.activation_unfinished += other.activation_unfinished;
        self.payment_completed += other.payment_completed;
        self.payment_cancelled += other.payment_cancelled;
        self.nothing_in_window += other.nothing_in_window;
        merge(&mut self.steps_to_start, &other.steps_to_start);
        merge(&mut self.cast_step_subtype, &other.cast_step_subtype);
        self.cast_step_n_ne_r += other.cast_step_n_ne_r;
        self.cast_step_n_eq_r += other.cast_step_n_eq_r;
        merge(&mut self.spells, &other.spells);
    }
}

/// One subtype's out-of-scope `N ≠ R` rows.
#[derive(Clone, Debug, Default, Serialize)]
struct SubtypeTally {
    n_ne_r: u64,
    r_acts_n_passes: u64,
    n_acts_r_passes: u64,
    both_act_same_kind: u64,
    both_act_other_kind: u64,
    undecoded: u64,
    /// Rows where `R`'s action is a mana ability (an `activate` row whose
    /// ability makes mana counts as a tap).
    r_mana_ability: u64,
    /// `R`'s action kind → `N`'s, at the both-act rows of another kind.
    other_kind_pairs: BTreeMap<String, u64>,
    /// What `R`'s chain did after the `R`-acts rows.
    r_acts_chain: ChainTally,
    /// The sources or abilities `R` used at the `R`-acts rows.
    r_names: BTreeMap<String, u64>,
}

impl SubtypeTally {
    fn add(&mut self, other: &Self) {
        self.n_ne_r += other.n_ne_r;
        self.r_acts_n_passes += other.r_acts_n_passes;
        self.n_acts_r_passes += other.n_acts_r_passes;
        self.both_act_same_kind += other.both_act_same_kind;
        self.both_act_other_kind += other.both_act_other_kind;
        self.undecoded += other.undecoded;
        self.r_mana_ability += other.r_mana_ability;
        merge(&mut self.other_kind_pairs, &other.other_kind_pairs);
        self.r_acts_chain.add(&other.r_acts_chain);
        merge(&mut self.r_names, &other.r_names);
    }
}

/// The `pass` rows: `R` passes, `N` acts.
#[derive(Clone, Debug, Default, Serialize)]
struct PassTally {
    rows: u64,
    /// `N`'s action kind.
    net_kind: BTreeMap<String, u64>,
    /// Of `N`'s mana taps: in the seat's own turn, and in the other side's.
    net_taps_own_turn: u64,
    net_taps_their_turn: u64,
}

impl PassTally {
    fn add(&mut self, other: &Self) {
        self.rows += other.rows;
        merge(&mut self.net_kind, &other.net_kind);
        self.net_taps_own_turn += other.net_taps_own_turn;
        self.net_taps_their_turn += other.net_taps_their_turn;
    }
}

/// Everything a run of the audit counts.
#[derive(Clone, Debug, Default, Serialize)]
struct Tally {
    games: u64,
    games_failed: u64,
    labelled: u64,
    n_ne_r: u64,
    by_subtype: BTreeMap<String, SubtypeTally>,
    pass: PassTally,
    examples: BTreeMap<String, Vec<Value>>,
}

impl Tally {
    fn add(&mut self, other: &Self, keep: usize) {
        self.games += other.games;
        self.games_failed += other.games_failed;
        self.labelled += other.labelled;
        self.n_ne_r += other.n_ne_r;
        for (name, tally) in &other.by_subtype {
            self.by_subtype.entry(name.clone()).or_default().add(tally);
        }
        self.pass.add(&other.pass);
        for (class, list) in &other.examples {
            let mine = self.examples.entry(class.clone()).or_default();
            for example in list {
                if mine.len() < keep {
                    mine.push(example.clone());
                }
            }
        }
    }

    fn example(&mut self, class: &str, keep: usize, value: impl FnOnce() -> Value) {
        let list = self.examples.entry(class.to_owned()).or_default();
        if list.len() < keep {
            list.push(value());
        }
    }
}

/// The seat's next inputs, for an example.
fn sequel(moves: &[Move], at: usize, window: usize) -> Vec<String> {
    moves
        .iter()
        .skip(at + 1)
        .take(window)
        .map(|m| {
            let kind = m.action.as_ref().map_or_else(|| "?".to_owned(), variant);
            let owed = if m.owed { " (paying)" } else { "" };
            match &m.name {
                Some(name) => format!("{kind} {name}{owed}"),
                None => format!("{kind}{owed}"),
            }
        })
        .collect()
}

/// Counts one seat's inputs of one game.
fn audit_seat(moves: &[Move], window: usize, keep: usize, game: &str, tally: &mut Tally) {
    for (at, m) in moves.iter().enumerate() {
        let Some(label) = &m.label else {
            continue;
        };
        tally.labelled += 1;
        if !label.n_ne_r {
            continue;
        }
        tally.n_ne_r += 1;
        let Some(subtype) = label.subtype else {
            continue;
        };
        let example = || {
            json!({"game": game, "input": at, "my_turn": m.my_turn, "owed": m.owed,
                "R": label.rules.as_ref().map(variant), "R_name": label.rules_name,
                "N": label.net.as_ref().map(variant), "N_name": label.net_name,
                "next": sequel(moves, at, window)})
        };
        if subtype == DistillSubtype4::Pass {
            tally.pass.rows += 1;
            let kind = label.net.as_ref().map_or_else(|| "?".to_owned(), variant);
            if matches!(label.net, Some(PlayerAction::ActivateManaAbility { .. })) {
                if m.my_turn {
                    tally.pass.net_taps_own_turn += 1;
                } else {
                    tally.pass.net_taps_their_turn += 1;
                }
                tally.example("pass: N taps mana", keep, example);
            }
            bump(&mut tally.pass.net_kind, kind);
            continue;
        }
        if !matches!(
            subtype,
            DistillSubtype4::Mana | DistillSubtype4::Pay | DistillSubtype4::Activate
        ) {
            continue;
        }
        let class = classify(label);
        let row = tally
            .by_subtype
            .entry(subtype.name().to_owned())
            .or_default();
        row.n_ne_r += 1;
        if label.rules_tap {
            row.r_mana_ability += 1;
        }
        match class {
            Class::RActsNPasses => {
                row.r_acts_n_passes += 1;
                if let Some(name) = &label.rules_name {
                    bump(&mut row.r_names, name.clone());
                }
                let followed = row.r_acts_chain.count(moves, at, window);
                let key = format!("{}: R acts, N passes: {followed:?}", subtype.name());
                let key = key.split(" {").next().unwrap_or_default().to_owned();
                tally.example(&key, keep, example);
            }
            Class::NActsRPasses => {
                row.n_acts_r_passes += 1;
                tally.example(
                    &format!("{}: N acts, R passes", subtype.name()),
                    keep,
                    example,
                );
            }
            Class::BothActSameKind => {
                row.both_act_same_kind += 1;
                tally.example(
                    &format!("{}: both act, same kind", subtype.name()),
                    keep,
                    example,
                );
            }
            Class::BothActOtherKind => {
                row.both_act_other_kind += 1;
                let pair = format!(
                    "{} -> {}",
                    label
                        .rules
                        .as_ref()
                        .map_or_else(|| "?".to_owned(), |a| kind(a, label.rules_tap)),
                    label
                        .net
                        .as_ref()
                        .map_or_else(|| "?".to_owned(), |a| kind(a, label.net_tap))
                );
                bump(&mut row.other_kind_pairs, pair);
                tally.example(
                    &format!("{}: both act, other kind", subtype.name()),
                    keep,
                    example,
                );
            }
            Class::Undecoded => row.undecoded += 1,
        }
    }
}

/// The name of what an action names, as the seat saw it.
fn name_of(view: &PlayerView, action: &PlayerAction) -> Option<String> {
    let object = |id| view.object(id).map(|o| o.name.to_string());
    match action {
        PlayerAction::CastSpell { card } | PlayerAction::PlayLand { card } => view
            .hand
            .iter()
            .find(|h| h.id == *card)
            .map(|h| h.name.clone())
            .or_else(|| object(*card)),
        PlayerAction::ActivateManaAbility { source }
        | PlayerAction::ActivateAbility { source, .. } => object(*source),
        _ => None,
    }
}

/// One game of `games.jsonl`.
struct Game {
    run: PathBuf,
    entry: Value,
}

/// Replays one game and returns each recorded seat's inputs.
fn moves_of(game: &Game) -> anyhow::Result<BTreeMap<u8, Vec<Move>>> {
    let shard = game.entry["shard"]
        .as_str()
        .context("a game without a shard")?;
    let offset = game.entry["offset"]
        .as_u64()
        .context("a game without an offset")?;
    let len = game.entry["len"].as_u64().context("a game without a len")?;
    let mut file = File::open(game.run.join("records").join(shard))?;
    file.seek(SeekFrom::Start(offset))?;
    let mut record = Vec::new();
    GzDecoder::new(file.take(len)).read_to_end(&mut record)?;
    let steps: Vec<DistillStep4> = serde_json::from_value(
        game.entry
            .get("distill_sidecar")
            .context("a game without distill_sidecar")?
            .clone(),
    )?;
    let mut session = None;
    let mut seat_input = 0_usize;
    let mut seats: BTreeMap<u8, Vec<Move>> = BTreeMap::new();
    for raw in record.split(|b| *b == b'\n').filter(|l| !l.is_empty()) {
        match serde_json::from_slice::<Line>(raw)? {
            Line::Header { preset, .. } => {
                let mut built = Session::new(&preset).context("the preset builds no session")?;
                for s in 0..preset.seats.len() {
                    built.take_over(PlayerId::new(u8::try_from(s)?));
                }
                session = Some(built);
            }
            Line::Input {
                seat, by, action, ..
            } => {
                let session = session.as_mut().context("an input before the header")?;
                let player = PlayerId::new(seat);
                if action.is_automation_setting() || !matches!(by, Source::Seat) {
                    let _ = session.act(player, action);
                    continue;
                }
                let step = steps
                    .get(seat_input)
                    .with_context(|| format!("no distillation step for seat input {seat_input}"))?;
                ensure!(
                    step.seat == seat,
                    "seat input {seat_input} is seat {}'s in the sidecar, seat {seat}'s in the record",
                    step.seat
                );
                seat_input += 1;
                let (_, view) = session
                    .view_for_with_context(player)
                    .context("a seat input without a question")?;
                let label = step.label.as_ref().map(|l| Label {
                    subtype: l.subtype,
                    n_ne_r: l.n_ne_r,
                    rules_name: l.rules_action.as_ref().and_then(|a| name_of(&view, a)),
                    net_name: l.net_action.as_ref().and_then(|a| name_of(&view, a)),
                    rules_tap: l.rules_action.as_ref().is_some_and(|a| is_tap(&view, a)),
                    net_tap: l.net_action.as_ref().is_some_and(|a| is_tap(&view, a)),
                    rules: l.rules_action.clone(),
                    net: l.net_action.clone(),
                });
                seats.entry(seat).or_default().push(Move {
                    name: name_of(&view, &action),
                    tap: is_tap(&view, &action),
                    owed: view.owed.is_some(),
                    my_turn: view.active == player,
                    stack: view.stack.iter().map(|o| o.name.to_string()).collect(),
                    action: Some(action.clone()),
                    label,
                });
                let _ = session.act(player, action);
            }
            Line::Chair { .. } | Line::DeclaredMind { .. } | Line::End { .. } => {}
        }
    }
    ensure!(
        seat_input == steps.len(),
        "the record has {seat_input} seat inputs, the sidecar {}",
        steps.len()
    );
    Ok(seats)
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    let mut games = Vec::new();
    for run in &args.run {
        let file = File::open(run.join("games.jsonl"))
            .with_context(|| format!("{}/games.jsonl", run.display()))?;
        for line in BufReader::new(file).lines() {
            let entry: Value = serde_json::from_str(&line?)?;
            if entry.get("distill_sidecar").is_some() {
                games.push(Game {
                    run: run.clone(),
                    entry,
                });
            }
        }
    }
    eprintln!("[scope_audit4] {} games", games.len());
    let next = AtomicUsize::new(0);
    let total = Mutex::new(Tally::default());
    let failures = Mutex::new(Vec::new());
    std::thread::scope(|scope| {
        for _ in 0..args.threads.max(1) {
            scope.spawn(|| {
                let mut mine = Tally::default();
                loop {
                    let index = next.fetch_add(1, Ordering::Relaxed);
                    let Some(game) = games.get(index) else {
                        break;
                    };
                    let name = format!(
                        "{}#{}",
                        game.run.file_name().unwrap_or_default().to_string_lossy(),
                        game.entry["i"]
                    );
                    match moves_of(game) {
                        Ok(seats) => {
                            mine.games += 1;
                            for moves in seats.values() {
                                audit_seat(moves, args.window, args.examples, &name, &mut mine);
                            }
                        }
                        Err(error) => {
                            mine.games_failed += 1;
                            failures
                                .lock()
                                .expect("failure lock")
                                .push(format!("{name}: {error:#}"));
                        }
                    }
                    if (index + 1).is_multiple_of(500) {
                        eprintln!("[scope_audit4] {} games read", index + 1);
                    }
                }
                total.lock().expect("tally lock").add(&mine, args.examples);
            });
        }
    });
    let tally = total.into_inner().expect("tally lock");
    let failures = failures.into_inner().expect("failure lock");
    let report = json!({"runs": args.run, "window": args.window, "tally": tally,
        "failures": failures.iter().take(20).collect::<Vec<_>>()});
    std::fs::write(&args.out, serde_json::to_vec_pretty(&report)?)?;
    eprintln!(
        "[scope_audit4] {} games read, {} failed; report {}",
        tally.games,
        tally.games_failed,
        args.out.display()
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use baylee_core::ids::ObjectId;

    fn id(n: u32) -> ObjectId {
        ObjectId::new(n, 0)
    }

    fn tap(n: u32) -> PlayerAction {
        PlayerAction::ActivateManaAbility { source: id(n) }
    }

    fn cast(n: u32) -> PlayerAction {
        PlayerAction::CastSpell { card: id(n) }
    }

    fn mv(action: PlayerAction, name: &str, owed: bool, stack: &[&str]) -> Move {
        Move {
            tap: matches!(action, PlayerAction::ActivateManaAbility { .. }),
            action: Some(action),
            name: (!name.is_empty()).then(|| name.to_owned()),
            owed,
            my_turn: true,
            stack: stack.iter().map(|s| (*s).to_owned()).collect(),
            label: None,
        }
    }

    fn labelled(
        mut m: Move,
        subtype: DistillSubtype4,
        rules: PlayerAction,
        net: PlayerAction,
    ) -> Move {
        let rules_tap = matches!(rules, PlayerAction::ActivateManaAbility { .. });
        let net_tap = matches!(net, PlayerAction::ActivateManaAbility { .. });
        m.label = Some(Label {
            subtype: Some(subtype),
            n_ne_r: rules != net,
            rules: Some(rules),
            net: Some(net),
            rules_name: m.name.clone(),
            net_name: None,
            rules_tap,
            net_tap,
        });
        m
    }

    #[test]
    fn a_disagreement_is_pass_against_act_or_act_against_act() {
        let label = |r, n| Label {
            rules: Some(r),
            net: Some(n),
            ..Label::default()
        };
        assert_eq!(
            classify(&label(tap(1), PlayerAction::PassPriority)),
            Class::RActsNPasses
        );
        assert_eq!(
            classify(&label(PlayerAction::CancelCast, tap(1))),
            Class::NActsRPasses
        );
        assert_eq!(classify(&label(tap(1), tap(2))), Class::BothActSameKind);
        assert_eq!(classify(&label(tap(1), cast(3))), Class::BothActOtherKind);
        assert_eq!(classify(&Label::default()), Class::Undecoded);
    }

    #[test]
    fn a_tap_that_starts_a_cast_is_followed_to_the_stack() {
        // Tap, tap, cast Shock, choose its target, pay nothing more, then
        // the seat holds priority with Shock on the stack.
        let moves = vec![
            mv(tap(1), "Mountain", false, &[]),
            mv(tap(2), "Mountain", false, &[]),
            mv(cast(9), "Shock", false, &[]),
            mv(PlayerAction::PassPriority, "", false, &["Shock"]),
        ];
        assert_eq!(
            chain(&moves, 0, 6),
            Chain::Completed {
                start: 2,
                cast: true
            }
        );
        // Beyond the window nothing starts.
        assert_eq!(chain(&moves, 0, 1), Chain::Nothing);
        // A cast cancelled before it is seen on the stack.
        let cancelled = vec![
            mv(tap(1), "Island", false, &[]),
            mv(cast(9), "Counterspell", false, &[]),
            mv(PlayerAction::CancelCast, "", true, &[]),
        ];
        assert_eq!(
            chain(&cancelled, 0, 6),
            Chain::Cancelled {
                start: 1,
                cast: true
            }
        );
        // A payment in progress: completes when the seat no longer owes.
        let paying = vec![
            mv(tap(1), "Forest", true, &[]),
            mv(tap(2), "Forest", true, &[]),
            mv(PlayerAction::PassPriority, "", false, &["Giant Growth"]),
        ];
        assert_eq!(chain(&paying, 0, 6), Chain::PaymentCompleted);
        // A mana ability taken through ActivateAbility (a dual land's colour)
        // is a tap of the chain: the cast after it is the start.
        let mut dual = mv(
            PlayerAction::ActivateAbility {
                source: id(3),
                ability_index: 0,
            },
            "Tranquil Cove",
            false,
            &[],
        );
        dual.tap = true;
        let through_dual = vec![
            mv(tap(1), "Plains", false, &[]),
            dual,
            mv(
                PlayerAction::ChooseColor(baylee_core::mana::ManaColor::White),
                "",
                false,
                &[],
            ),
            mv(cast(9), "Moorland Inquisitor", false, &[]),
            mv(
                PlayerAction::PassPriority,
                "",
                false,
                &["Moorland Inquisitor"],
            ),
        ];
        assert_eq!(
            chain(&through_dual, 0, 6),
            Chain::Completed {
                start: 3,
                cast: true
            }
        );
        // A tap followed only by passes.
        let idle = vec![
            mv(tap(1), "Plains", false, &[]),
            mv(PlayerAction::PassPriority, "", false, &[]),
        ];
        assert_eq!(chain(&idle, 0, 6), Chain::Nothing);
    }

    #[test]
    fn the_audit_counts_classes_chains_and_the_cast_decisions_label() {
        let moves = vec![
            labelled(
                mv(tap(1), "Mountain", false, &[]),
                DistillSubtype4::Mana,
                tap(1),
                PlayerAction::PassPriority,
            ),
            labelled(
                mv(cast(9), "Shock", false, &[]),
                DistillSubtype4::Cast,
                cast(9),
                cast(9),
            ),
            mv(PlayerAction::PassPriority, "", false, &["Shock"]),
            labelled(
                mv(tap(2), "Mountain", false, &[]),
                DistillSubtype4::Mana,
                tap(2),
                tap(3),
            ),
            labelled(
                mv(PlayerAction::PassPriority, "", false, &[]),
                DistillSubtype4::Pass,
                PlayerAction::PassPriority,
                tap(4),
            ),
        ];
        let mut tally = Tally::default();
        audit_seat(&moves, 6, 2, "g", &mut tally);
        assert_eq!((tally.labelled, tally.n_ne_r), (4, 3));
        let mana = &tally.by_subtype["mana"];
        assert_eq!(
            (mana.n_ne_r, mana.r_acts_n_passes, mana.both_act_same_kind),
            (2, 1, 1)
        );
        assert_eq!(mana.r_acts_chain.cast_completed, 1);
        assert_eq!(mana.r_acts_chain.cast_step_subtype["cast"], 1);
        assert_eq!(mana.r_acts_chain.cast_step_n_eq_r, 1);
        assert_eq!(mana.r_acts_chain.spells["Shock"], 1);
        assert_eq!(mana.r_acts_chain.steps_to_start["1"], 1);
        assert_eq!((tally.pass.rows, tally.pass.net_taps_own_turn), (1, 1));
        assert_eq!(tally.pass.net_kind["ActivateManaAbility"], 1);
    }
}
