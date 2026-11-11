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
use baylee_engine::choice::{Pending, PlayerAction};
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
    /// The rule groups that changed something on the way to `R`.
    fired: Vec<String>,
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
    /// The turn number and the phase and step (`Main` alone names both main
    /// phases) at the decision.
    turn: u32,
    when: String,
    /// At priority: what the menu offered beyond passing and mana.
    offered: Option<Offered>,
    /// The names on the stack at the decision.
    stack: Vec<String>,
    /// The decision's label, when the policy answered it.
    label: Option<Label>,
}

/// What a priority menu offered: spells castable now (mana floating),
/// spells castable by paying in a window (`payable`), non-mana abilities.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct Offered {
    castable: Vec<String>,
    payable: Vec<String>,
    abilities: Vec<String>,
}

impl Offered {
    fn of(view: &PlayerView, pending: &Pending) -> Option<Self> {
        let Pending::Priority { legal, .. } = pending else {
            return None;
        };
        let names = |ids: &[baylee_core::ids::ObjectId]| -> Vec<String> {
            ids.iter()
                .filter_map(|id| name_of(view, &PlayerAction::CastSpell { card: *id }))
                .collect()
        };
        let abilities = legal
            .abilities
            .iter()
            .map(|&(source, ability_index)| PlayerAction::ActivateAbility {
                source,
                ability_index,
            })
            .filter(|action| !is_tap(view, action))
            .filter_map(|action| name_of(view, &action))
            .collect();
        Some(Self {
            castable: names(&legal.castable),
            payable: names(&legal.payable),
            abilities,
        })
    }

    /// Which kinds the menu held.
    fn kinds(&self) -> String {
        let mut kinds = Vec::new();
        if !self.castable.is_empty() {
            kinds.push("castable");
        }
        if !self.payable.is_empty() {
            kinds.push("payable");
        }
        if !self.abilities.is_empty() {
            kinds.push("ability");
        }
        if kinds.is_empty() {
            "nothing to cast or activate".to_owned()
        } else {
            kinds.join(" + ")
        }
    }
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

/// What followed a decision where `R` passed and `N` would have tapped.
#[derive(Clone, Debug, PartialEq, Eq)]
struct PassFollow {
    /// The seat's input (index) and kind (`true`: a cast) of the cast or
    /// activation that started within the window, if one did.
    start: Option<(usize, bool)>,
    /// `completed` (seen on the stack), `cancelled`, `unfinished` or `nothing`.
    outcome: &'static str,
    /// How it was paid: taps before it, taps in its payment window, or none.
    path: &'static str,
    /// When it started: the pass's step, a later step of its turn, a later turn.
    started: &'static str,
    /// Where the pass led: the seat's next decision.
    led_to: String,
}

/// Where a pass at `here` led, read from the seat's next decision.
fn led_to(here: &Move, next: Option<&Move>) -> String {
    match next {
        None => "the game ends".to_owned(),
        Some(next) if next.turn != here.turn && next.my_turn => "the seat's next turn".to_owned(),
        Some(next) if next.turn != here.turn => "the other side's turn".to_owned(),
        Some(next)
            if matches!(
                next.action,
                Some(PlayerAction::DeclareAttackers { .. } | PlayerAction::DeclareBlockers { .. })
            ) =>
        {
            "combat: a declaration".to_owned()
        }
        Some(next) if next.when != here.when => format!("next step: {}", next.when),
        Some(_) => "the same step: priority again".to_owned(),
    }
}

/// Follows the seat's inputs after `moves[at]`, where `R` passed.
fn follow_pass(moves: &[Move], at: usize, window: usize) -> PassFollow {
    const LOOK: usize = 24;
    let here = &moves[at];
    let led_to = led_to(here, moves.get(at + 1));
    let starts = |m: &Move| {
        matches!(m.action, Some(PlayerAction::CastSpell { .. }))
            || (matches!(m.action, Some(PlayerAction::ActivateAbility { .. })) && !m.tap)
    };
    let Some(start) = (at + 1..moves.len().min(at + 1 + window)).find(|&j| starts(&moves[j]))
    else {
        return PassFollow {
            start: None,
            outcome: "nothing",
            path: "",
            started: "",
            led_to,
        };
    };
    let cast = matches!(moves[start].action, Some(PlayerAction::CastSpell { .. }));
    let tapped_before = moves[at + 1..start].iter().any(|m| m.tap);
    let tapped_paying = moves
        .iter()
        .skip(start + 1)
        .take_while(|m| m.owed)
        .any(|m| m.tap);
    let path = match (tapped_before, tapped_paying) {
        (false, false) => "no tap: paid automatically or free",
        (true, false) => "taps first, then the cast or activation",
        (false, true) => "taps in its payment window",
        (true, true) => "taps first and in its payment window",
    };
    let started = if moves[start].turn != here.turn {
        "a later turn"
    } else if moves[start].when == here.when {
        "the pass's own step"
    } else {
        "a later step of the same turn"
    };
    let name = moves[start].name.clone();
    let mut outcome = "unfinished";
    for later in moves.iter().skip(start + 1).take(LOOK) {
        if matches!(later.action, Some(PlayerAction::CancelCast)) {
            outcome = "cancelled";
            break;
        }
        if name.as_ref().is_some_and(|n| later.stack.contains(n)) {
            outcome = "completed";
            break;
        }
    }
    PassFollow {
        start: Some((start, cast)),
        outcome,
        path,
        started,
        led_to,
    }
}

/// The rows where `R` passes and `N` would have tapped, over all subtypes.
#[derive(Clone, Debug, Default, Serialize)]
struct PassAudit {
    rows: u64,
    own_turn: u64,
    by_subtype: BTreeMap<String, u64>,
    /// `cast completed`, `activation completed`, `... cancelled`,
    /// `... unfinished`, or `nothing` within the window.
    outcome: BTreeMap<String, u64>,
    /// Of the completed: how they were paid, and when they started.
    path: BTreeMap<String, u64>,
    started: BTreeMap<String, u64>,
    /// Where the pass led.
    led_to: BTreeMap<String, u64>,
    /// What the menu offered at the pass (spells castable now, payable in a
    /// window, non-mana abilities), and that against what followed.
    offered: BTreeMap<String, u64>,
    offered_then: BTreeMap<String, u64>,
    /// The groups that fired there (`none`: the guard's own reading).
    fired: BTreeMap<String, u64>,
}

impl PassAudit {
    fn add(&mut self, other: &Self) {
        self.rows += other.rows;
        self.own_turn += other.own_turn;
        merge(&mut self.by_subtype, &other.by_subtype);
        merge(&mut self.outcome, &other.outcome);
        merge(&mut self.path, &other.path);
        merge(&mut self.started, &other.started);
        merge(&mut self.led_to, &other.led_to);
        merge(&mut self.offered, &other.offered);
        merge(&mut self.offered_then, &other.offered_then);
        merge(&mut self.fired, &other.fired);
    }

    fn count(
        &mut self,
        subtype: Option<DistillSubtype4>,
        own_turn: bool,
        offered: Option<&Offered>,
        fired: &[String],
        follow: &PassFollow,
    ) {
        if fired.is_empty() {
            bump(&mut self.fired, "none");
        }
        for group in fired {
            bump(&mut self.fired, group.clone());
        }
        let kinds = offered.map_or_else(|| "not a priority menu".to_owned(), Offered::kinds);
        bump(&mut self.offered, kinds.clone());
        let then = if follow.outcome == "completed" {
            "a cast or activation reached the stack within the window"
        } else {
            "none within the window"
        };
        bump(&mut self.offered_then, format!("{kinds} -> {then}"));
        self.rows += 1;
        self.own_turn += u64::from(own_turn);
        bump(
            &mut self.by_subtype,
            subtype.map_or("none", DistillSubtype4::name),
        );
        let what = match follow.start {
            None => "nothing".to_owned(),
            Some((_, cast)) => format!(
                "{} {}",
                if cast { "cast" } else { "activation" },
                follow.outcome
            ),
        };
        bump(&mut self.outcome, what);
        if follow.outcome == "completed" {
            bump(&mut self.path, follow.path);
            bump(&mut self.started, follow.started);
        }
        bump(&mut self.led_to, follow.led_to.clone());
    }
}

/// The sample's seed and its rows per run: own turn, the other side's.
const SAMPLE_SEED: u64 = 0x5eed_0000_0000_0050;
const SAMPLE_OWN: usize = 10;
const SAMPLE_THEIRS: usize = 3;

fn sample_quota(group: &str) -> usize {
    if group.ends_with("/own") {
        SAMPLE_OWN
    } else {
        SAMPLE_THEIRS
    }
}

/// A stable hash for drawing the sample (FNV-1a over the seed and the row's
/// key), the same whatever order the games are read in.
fn sample_hash(seed: u64, key: &str) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325_u64 ^ seed;
    for byte in key.bytes() {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x0100_0000_01b3);
    }
    hash
}

/// Keeps the `keep` rows of lowest hash in a group of the sample.
fn keep_lowest(list: &mut Vec<(u64, Value)>, keep: usize, hash: u64, row: impl FnOnce() -> Value) {
    if list.len() >= keep && list.last().is_some_and(|(worst, _)| *worst <= hash) {
        return;
    }
    let at = list.partition_point(|(h, _)| *h < hash);
    list.insert(at, (hash, row()));
    list.truncate(keep);
}

/// The seat's next inputs with their steps and the stack, for a sample row.
fn sequel_with_steps(moves: &[Move], at: usize, window: usize) -> Vec<String> {
    use std::fmt::Write as _;
    let (mut when, mut turn) = (moves[at].when.clone(), moves[at].turn);
    moves
        .iter()
        .skip(at + 1)
        .take(window)
        .map(|m| {
            let mut entry = String::new();
            if m.turn != turn {
                let whose = if m.my_turn { "own" } else { "theirs" };
                let _ = write!(entry, "[turn {} {whose}, {}] ", m.turn, m.when);
            } else if m.when != when {
                let _ = write!(entry, "[{}] ", m.when);
            }
            (when, turn) = (m.when.clone(), m.turn);
            entry.push_str(&m.action.as_ref().map_or_else(|| "?".to_owned(), variant));
            if let Some(name) = &m.name {
                let _ = write!(entry, " {name}");
            }
            if m.owed {
                entry.push_str(" (paying)");
            }
            if !m.stack.is_empty() {
                let _ = write!(entry, " {{stack: {}}}", m.stack.join(", "));
            }
            entry
        })
        .collect()
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
    /// Every row where `R` passes and `N` would have tapped (coarse classes).
    r_pass_n_tap: PassAudit,
    /// The seeded sample of those rows, by run and own turn or not.
    #[serde(skip)]
    sample: BTreeMap<String, Vec<(u64, Value)>>,
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
        self.r_pass_n_tap.add(&other.r_pass_n_tap);
        for (group, rows) in &other.sample {
            let mine = self.sample.entry(group.clone()).or_default();
            let quota = sample_quota(group);
            for (hash, row) in rows {
                keep_lowest(mine, quota, *hash, || row.clone());
            }
        }
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

/// Counts a row where `R` passes and `N` would have tapped, and offers it to
/// the sample.
fn audit_pass(
    moves: &[Move],
    at: usize,
    window: usize,
    game: &str,
    label: &Label,
    tally: &mut Tally,
) {
    let m = &moves[at];
    let follow = follow_pass(moves, at, window);
    tally.r_pass_n_tap.count(
        label.subtype,
        m.my_turn,
        m.offered.as_ref(),
        &label.fired,
        &follow,
    );
    let run = game.split('#').next().unwrap_or(game);
    let group = format!("{run}/{}", if m.my_turn { "own" } else { "theirs" });
    let hash = sample_hash(SAMPLE_SEED, &format!("{game}/{at}"));
    let list = tally.sample.entry(group.clone()).or_default();
    keep_lowest(list, sample_quota(&group), hash, || {
        let start = follow.start.map(|(s, _)| {
            let start = &moves[s];
            format!(
                "{} {}",
                start
                    .action
                    .as_ref()
                    .map_or_else(|| "?".to_owned(), variant),
                start.name.clone().unwrap_or_default()
            )
        });
        json!({"game": game, "input": at, "turn": m.turn, "when": m.when, "own_turn": m.my_turn,
            "subtype": label.subtype.map(DistillSubtype4::name), "R": label.rules.as_ref().map(variant),
            "N_taps": label.net_name, "fired": label.fired, "outcome": follow.outcome, "start": start, "path": follow.path,
            "started": follow.started, "led_to": follow.led_to,
            "offered": m.offered.as_ref().map(|o| json!({"castable": o.castable, "payable": o.payable, "abilities": o.abilities})),
            "next": sequel_with_steps(moves, at, window)})
    });
}

/// Counts a `pass` row: what `N` would have done where `R` passed.
fn count_pass_subtype(
    tally: &mut Tally,
    label: &Label,
    own_turn: bool,
    keep: usize,
    example: impl FnOnce() -> Value,
) {
    tally.pass.rows += 1;
    let kind = label.net.as_ref().map_or_else(|| "?".to_owned(), variant);
    if matches!(label.net, Some(PlayerAction::ActivateManaAbility { .. })) {
        if own_turn {
            tally.pass.net_taps_own_turn += 1;
        } else {
            tally.pass.net_taps_their_turn += 1;
        }
        tally.example("pass: N taps mana", keep, example);
    }
    bump(&mut tally.pass.net_kind, kind);
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
        if matches!(
            label.rules,
            Some(PlayerAction::PassPriority | PlayerAction::CancelCast)
        ) && label.net_tap
        {
            audit_pass(moves, at, window, game, label, tally);
        }
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
            count_pass_subtype(tally, label, m.my_turn, keep, example);
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
                let (pending, view) = session
                    .view_for_with_context(player)
                    .context("a seat input without a question")?;
                let label = step.label.as_ref().map(|l| Label {
                    subtype: l.subtype,
                    n_ne_r: l.n_ne_r,
                    rules_name: l.rules_action.as_ref().and_then(|a| name_of(&view, a)),
                    net_name: l.net_action.as_ref().and_then(|a| name_of(&view, a)),
                    rules_tap: l.rules_action.as_ref().is_some_and(|a| is_tap(&view, a)),
                    net_tap: l.net_action.as_ref().is_some_and(|a| is_tap(&view, a)),
                    fired: l.fired.clone(),
                    rules: l.rules_action.clone(),
                    net: l.net_action.clone(),
                });
                seats.entry(seat).or_default().push(Move {
                    name: name_of(&view, &action),
                    tap: is_tap(&view, &action),
                    owed: view.owed.is_some(),
                    my_turn: view.active == player,
                    turn: view.turn,
                    when: format!("{:?}/{:?}", view.phase, view.step),
                    offered: Offered::of(&view, &pending),
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
                            for (seat, moves) in &seats {
                                let key = format!("{name}/seat{seat}");
                                audit_seat(moves, args.window, args.examples, &key, &mut mine);
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
    let sample: Vec<&Value> = tally
        .sample
        .values()
        .flatten()
        .map(|(_, row)| row)
        .collect();
    let report = json!({"runs": args.run, "window": args.window, "tally": tally,
        "r_pass_n_tap_sample": sample,
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
            turn: 1,
            when: "PrecombatMain/Main".to_owned(),
            offered: None,
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
            fired: vec![],
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
    fn a_pass_is_followed_to_where_it_led_and_what_reached_the_stack() {
        let at = |mut m: Move, turn: u32, when: &str, mine: bool| {
            m.turn = turn;
            m.when = when.to_owned();
            m.my_turn = mine;
            m.offered = Some(Offered::default());
            m
        };
        // Passing in the first main phase, then tapping and casting after combat.
        let later = vec![
            at(
                mv(PlayerAction::PassPriority, "", false, &[]),
                3,
                "PrecombatMain/Main",
                true,
            ),
            at(
                mv(PlayerAction::PassPriority, "", false, &[]),
                3,
                "Combat/CombatBegin",
                true,
            ),
            at(
                mv(tap(1), "Forest", false, &[]),
                3,
                "PostcombatMain/Main",
                true,
            ),
            at(
                mv(cast(9), "Grizzly Bears", false, &[]),
                3,
                "PostcombatMain/Main",
                true,
            ),
            at(
                mv(PlayerAction::PassPriority, "", false, &["Grizzly Bears"]),
                3,
                "PostcombatMain/Main",
                true,
            ),
        ];
        let follow = follow_pass(&later, 0, 6);
        assert_eq!(follow.start, Some((3, true)));
        assert_eq!(follow.outcome, "completed");
        assert_eq!(follow.path, "taps first, then the cast or activation");
        assert_eq!(follow.started, "a later step of the same turn");
        assert_eq!(follow.led_to, "next step: Combat/CombatBegin");
        // A cast in the pass's own step with no tap: paid automatically.
        let autopay = vec![
            at(
                mv(PlayerAction::PassPriority, "", false, &[]),
                5,
                "Ending/End",
                false,
            ),
            at(
                mv(cast(4), "Shock", false, &["Llanowar Elves"]),
                5,
                "Ending/End",
                false,
            ),
            at(
                mv(PlayerAction::PassPriority, "", false, &["Shock"]),
                5,
                "Ending/End",
                false,
            ),
        ];
        let follow = follow_pass(&autopay, 0, 6);
        assert_eq!(
            (follow.outcome, follow.path, follow.started),
            (
                "completed",
                "no tap: paid automatically or free",
                "the pass's own step"
            )
        );
        assert_eq!(follow.led_to, "the same step: priority again");
        // A pass that ends the turn, nothing cast.
        let idle = vec![
            at(
                mv(PlayerAction::PassPriority, "", false, &[]),
                6,
                "Ending/End",
                true,
            ),
            at(
                mv(PlayerAction::PassPriority, "", false, &[]),
                7,
                "Beginning/Upkeep",
                false,
            ),
        ];
        let follow = follow_pass(&idle, 0, 6);
        assert_eq!(
            (follow.outcome, follow.led_to.as_str()),
            ("nothing", "the other side's turn")
        );
    }

    #[test]
    fn the_sample_keeps_the_lowest_hashes_whatever_the_order() {
        let mut list = Vec::new();
        for key in ["c", "a", "d", "b"] {
            keep_lowest(&mut list, 2, sample_hash(1, key), || json!(key));
        }
        let mut expected: Vec<(u64, &str)> = ["a", "b", "c", "d"]
            .iter()
            .map(|k| (sample_hash(1, k), *k))
            .collect();
        expected.sort_unstable();
        let kept: Vec<&str> = list.iter().map(|(_, v)| v.as_str().unwrap()).collect();
        assert_eq!(kept, [expected[0].1, expected[1].1]);
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
