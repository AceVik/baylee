//! Where the house search answers a block otherwise than the level's own
//! heuristics, read from a question log (`eval4 --house-foresight
//! --foresight-questions`): what a repair of the heuristics is mined from.
//!
//! Each logged question holds the seat's view, the question, every answer
//! tried with its value in each world, and the answer given. For a block
//! this tool sets the level's own blocks beside the search's and names what
//! changed, one change a question:
//!
//! - `added`: the search blocks with a creature the level left at home;
//! - `dropped`: the search leaves home a creature the level blocked with;
//! - `moved`: the same creatures block, one of them another attacker;
//! - `mixed`: more than one of these.
//!
//! and what the block that changed is, as the two creatures meet alone:
//! `chump` (the blocker dies, the attacker lives), `trade` (both die), `eat`
//! (the attacker dies, the blocker lives), `wall` (neither dies), or `gang`
//! where the attacker is blocked by more than one creature. With that, where
//! the seat stood: its life in a band, and whether the level's own blocks
//! would have let lethal damage through.
//!
//! A class is weighed by its *gain*: the mean, over the worlds both answers
//! were tried in, of the search's value of its answer less that of the
//! level's own, summed over the class. That is the search's own judgement,
//! in forks where a house player sits in every chair; the probe
//! (`foresight_probe`) is what checks it against games played out.
//!
//! `--ask <level>` asks a level's heuristics every logged question again
//! (a level with knobs, as `eval4` names one: `expert:generation=7`) and
//! says how often it now gives the search's answer where the search had
//! replaced the level's, how often it leaves an answer the search had let
//! stand, and what the search made of the answers it gives where it had
//! tried them. A repair is developed against that before any game is
//! played. `--show <n>` prints the first `n` questions of each class.

#![allow(clippy::missing_docs_in_private_items)]

use std::collections::BTreeMap;
use std::io::{BufRead as _, BufReader};
use std::path::PathBuf;

use anyhow::Context as _;
use baylee_ai::HeuristicAgent;
use baylee_cards_dsl::KeywordSet;
// The house looks cards up, so this binary holds the pool.
use baylee_cards_pool as _;
use baylee_core::ids::{Defender, ObjectId};
use baylee_engine::choice::{Pending, PlayerAction};
use baylee_view::{PlayerView, PublicObject};
use clap::Parser;

#[path = "eval4/profile_spec.rs"]
mod profile_spec;

#[derive(Parser)]
#[command(about = "Sets the house search's blocks beside the level's own, by what changed")]
struct Args {
    /// The question logs.
    #[arg(long, required = true, num_args = 1..)]
    log: Vec<PathBuf>,
    /// A level whose heuristics are asked every question again.
    #[arg(long)]
    ask: Option<String>,
    /// How many questions of each class are printed.
    #[arg(long, default_value_t = 0)]
    show: usize,
    /// Only the classes whose name contains this are printed by `--show`.
    #[arg(long)]
    class: Option<String>,
}

#[derive(serde::Deserialize)]
struct Tried {
    a: PlayerAction,
    v: Vec<Option<f64>>,
}

#[derive(serde::Deserialize)]
struct Question {
    view: PlayerView,
    pending: Pending,
    tried: Vec<Tried>,
    chosen: usize,
}

impl Question {
    /// The mean of answer `at`'s value less the level's own, over the
    /// worlds both were tried in.
    fn gain(&self, at: usize) -> f64 {
        let pairs: Vec<f64> = self.tried[0]
            .v
            .iter()
            .zip(&self.tried[at].v)
            .filter_map(|(own, other)| own.zip(*other).map(|(own, other)| other - own))
            .collect();
        if pairs.is_empty() {
            return 0.0;
        }
        #[allow(clippy::cast_precision_loss)]
        let mean = pairs.iter().sum::<f64>() / pairs.len() as f64;
        mean
    }
}

type Blocks = Vec<(ObjectId, ObjectId)>;

fn blocks(action: &PlayerAction) -> Blocks {
    let PlayerAction::DeclareBlockers { blockers } = action else {
        return Vec::new();
    };
    let mut blockers = blockers.clone();
    blockers.sort_unstable();
    blockers
}

fn has(object: &PublicObject, keyword: KeywordSet) -> bool {
    object.keywords & keyword.bits() != 0
}

fn first(object: &PublicObject) -> bool {
    has(
        object,
        KeywordSet::FIRST_STRIKE.union(KeywordSet::DOUBLE_STRIKE),
    )
}

fn power(object: &PublicObject) -> i32 {
    i32::from(object.power.unwrap_or(0)).max(0)
}

fn toughness(object: &PublicObject) -> i32 {
    i32::from(object.toughness.unwrap_or(0)) - i32::from(object.damage)
}

/// Whether `by`'s combat damage destroys `it`.
fn kills(by: &PublicObject, it: &PublicObject) -> bool {
    !has(it, KeywordSet::INDESTRUCTIBLE)
        && power(by) > 0
        && (has(by, KeywordSet::DEATHTOUCH) || power(by) >= toughness(it))
}

/// What `blocker` blocking `attacker` alone is: chump, trade, eat or wall.
fn exchange(attacker: &PublicObject, blocker: &PublicObject) -> &'static str {
    let mut blocker_dies = kills(attacker, blocker);
    let mut attacker_dies = kills(blocker, attacker);
    // A first strike that kills leaves nothing to strike back (CR 510.4).
    if first(attacker) && !first(blocker) && blocker_dies {
        attacker_dies = false;
    }
    if first(blocker) && !first(attacker) && attacker_dies {
        blocker_dies = false;
    }
    match (blocker_dies, attacker_dies) {
        (true, false) => "chump",
        (true, true) => "trade",
        (false, true) => "eat",
        (false, false) => "wall",
    }
}

fn describe(object: &PublicObject) -> String {
    let mut words = Vec::new();
    for (keyword, word) in [
        (KeywordSet::FLYING, "fly"),
        (KeywordSet::FIRST_STRIKE, "fs"),
        (KeywordSet::DOUBLE_STRIKE, "ds"),
        (KeywordSet::DEATHTOUCH, "dt"),
        (KeywordSet::TRAMPLE, "tr"),
        (KeywordSet::LIFELINK, "ll"),
        (KeywordSet::MENACE, "men"),
        (KeywordSet::VIGILANCE, "vig"),
        (KeywordSet::REACH, "reach"),
        (KeywordSet::INDESTRUCTIBLE, "ind"),
        (KeywordSet::DEFENDER, "def"),
    ] {
        if has(object, keyword) {
            words.push(word);
        }
    }
    format!(
        "{} {}/{}{}{} mv{}",
        object.name,
        power(object),
        toughness(object),
        if words.is_empty() { "" } else { " " },
        words.join(","),
        object.mana_value
    )
}

/// The damage `blocks` lets through to the seat, as the attackers' power
/// (trample over a block is not counted).
fn unblocked(view: &PlayerView, blocks: &Blocks) -> i32 {
    view.combat
        .attackers
        .iter()
        .filter(|attacker| attacker.defending == Defender::Player(view.seat))
        .filter(|attacker| {
            !blocks
                .iter()
                .any(|(_, blocked)| *blocked == attacker.creature)
        })
        .filter_map(|attacker| view.object(attacker.creature))
        .map(|object| {
            power(object)
                * if has(object, KeywordSet::DOUBLE_STRIKE) {
                    2
                } else {
                    1
                }
        })
        .sum()
}

/// What changed from the level's blocks to the search's, and the kind of
/// the block that changed.
fn change(view: &PlayerView, own: &Blocks, chosen: &Blocks) -> String {
    let added: Vec<_> = chosen.iter().filter(|pair| !own.contains(pair)).collect();
    let dropped: Vec<_> = own.iter().filter(|pair| !chosen.contains(pair)).collect();
    let kind = |pair: &(ObjectId, ObjectId), among: &Blocks| {
        let gang = among
            .iter()
            .filter(|(_, attacker)| *attacker == pair.1)
            .count()
            > 1;
        if gang {
            return "gang";
        }
        match (view.object(pair.1), view.object(pair.0)) {
            (Some(attacker), Some(blocker)) => exchange(attacker, blocker),
            _ => "unread",
        }
    };
    match (added.as_slice(), dropped.as_slice()) {
        ([pair], []) => format!("added {}", kind(pair, chosen)),
        ([], [pair]) => format!("dropped {}", kind(pair, own)),
        ([to], [from]) if to.0 == from.0 => {
            format!("moved {} to {}", kind(from, own), kind(to, chosen))
        }
        _ => "mixed".to_string(),
    }
}

/// Where the seat stood: its life in a band, and whether the level's own
/// blocks let lethal damage through.
fn standing(view: &PlayerView, own: &Blocks) -> String {
    let life = view.seat(view.seat).map_or(0, |seat| seat.life);
    let through = unblocked(view, own);
    let band = match life {
        ..=5 => "life<=5",
        6..=10 => "life 6-10",
        _ => "life>10",
    };
    if through >= life {
        format!("{band}, own lets lethal through")
    } else {
        band.to_string()
    }
}

fn print_question(question: &Question, own: &Blocks, chosen: &Blocks, asked: Option<&Blocks>) {
    let view = &question.view;
    let seat = |player| view.seat(player);
    let me = seat(view.seat);
    let other = view.seats.iter().find(|s| s.player != view.seat);
    println!(
        "  turn {} | me life {} hand {} | they life {} hand {}",
        view.turn,
        me.map_or(0, |s| s.life),
        view.hand.len(),
        other.map_or(0, |s| s.life),
        other.map_or(0, |s| s.hand_count),
    );
    for attacker in &view.combat.attackers {
        if let Some(object) = view.object(attacker.creature) {
            println!(
                "    attacks: [{:?}] {}",
                attacker.creature,
                describe(object)
            );
        }
    }
    let Pending::ChooseBlockers { blockers, .. } = &question.pending else {
        return;
    };
    for option in blockers {
        if let Some(object) = view.object(option.blocker) {
            println!("    may block: [{:?}] {}", option.blocker, describe(object));
        }
    }
    let others: Vec<String> = view
        .battlefield
        .iter()
        .filter(|object| object.controller != view.seat && object.power.is_some())
        .filter(|object| {
            !view
                .combat
                .attackers
                .iter()
                .any(|a| a.creature == object.id)
        })
        .map(describe)
        .collect();
    if !others.is_empty() {
        println!("    they keep home: {}", others.join("; "));
    }
    let name = |pairs: &Blocks| {
        if pairs.is_empty() {
            return "no block".to_string();
        }
        pairs
            .iter()
            .map(|(blocker, attacker)| {
                format!(
                    "{} x {}",
                    view.object(*blocker).map_or("?", |o| &*o.name),
                    view.object(*attacker).map_or("?", |o| &*o.name)
                )
            })
            .collect::<Vec<_>>()
            .join(", ")
    };
    println!("    level:  {}", name(own));
    println!(
        "    search: {} (gain {:+.3})",
        name(chosen),
        question.gain(question.chosen)
    );
    if let Some(asked) = asked {
        println!("    asked:  {}", name(asked));
    }
}

#[derive(Default)]
struct Class {
    questions: usize,
    gain: f64,
    /// Under `--ask`: the level asked gives the search's answer.
    repaired: usize,
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    let asked_level = args
        .ask
        .as_deref()
        .map(profile_spec::parse)
        .transpose()?
        .map(HeuristicAgent::new);
    let mut classes: BTreeMap<String, Class> = BTreeMap::new();
    let mut standings: BTreeMap<String, Class> = BTreeMap::new();
    let mut shown: BTreeMap<String, usize> = BTreeMap::new();
    let (mut questions, mut replaced) = (0_usize, 0_usize);
    // Under `--ask`: answers the search let stand that the level asked now
    // changes, how many of those the search had tried, and their gain.
    let (mut same, mut unsettled, mut known, mut known_gain) = (0_usize, 0_usize, 0_usize, 0.0);
    let (mut total_gain, mut asked_gain) = (0.0, 0.0);
    for path in &args.log {
        let file = std::fs::File::open(path).with_context(|| format!("{}", path.display()))?;
        for line in BufReader::new(file).lines() {
            let line = line?;
            let question: Question = serde_json::from_str(&line)
                .with_context(|| format!("{}: not a logged question", path.display()))?;
            if !matches!(question.pending, Pending::ChooseBlockers { .. })
                || question.tried.is_empty()
            {
                continue;
            }
            questions += 1;
            let own = blocks(&question.tried[0].a);
            let chosen = blocks(&question.tried[question.chosen].a);
            let asked = asked_level
                .as_ref()
                .map(|agent| blocks(&agent.act(&question.view, &question.pending)));
            let gain = question.gain(question.chosen);
            total_gain += gain;
            if let Some(asked) = &asked {
                // What the search made of the answer the level asked gives,
                // where it tried it.
                let at = question
                    .tried
                    .iter()
                    .position(|tried| blocks(&tried.a) == *asked);
                if let Some(at) = at {
                    asked_gain += question.gain(at);
                }
                if *asked == own {
                    same += 1;
                } else if question.chosen == 0 {
                    unsettled += 1;
                    if let Some(at) = at {
                        known += 1;
                        known_gain += question.gain(at);
                    }
                }
            }
            if question.chosen == 0 {
                continue;
            }
            replaced += 1;
            let class = change(&question.view, &own, &chosen);
            let repaired = asked.as_ref().is_some_and(|asked| *asked == chosen);
            for (name, into) in [
                (class.clone(), &mut classes),
                (standing(&question.view, &own), &mut standings),
            ] {
                let entry = into.entry(name).or_default();
                entry.questions += 1;
                entry.gain += gain;
                entry.repaired += usize::from(repaired);
            }
            let seen = shown.entry(class.clone()).or_default();
            if *seen < args.show && args.class.as_ref().is_none_or(|part| class.contains(part)) {
                *seen += 1;
                println!("{class} ({}):", standing(&question.view, &own));
                print_question(&question, &own, &chosen, asked.as_ref());
            }
        }
    }
    println!("{questions} blocks searched, {replaced} answered otherwise than the level");
    println!("the search's gain over them all: {total_gain:.2}");
    for (title, table) in [
        ("what changed", &classes),
        ("where the seat stood", &standings),
    ] {
        print_table(title, table);
    }
    if let Some(spec) = &args.ask {
        println!(
            "\n{spec}: the logged level's answer at {same} of {questions}; of the {} the search \
             let stand it changes {unsettled}, {known} to an answer the search tried (gain \
             {known_gain:+.2})",
            questions - replaced
        );
        println!(
            "{spec}: the search's gain of its answers where tried: {asked_gain:.2} of \
             {total_gain:.2}"
        );
    }
    Ok(())
}

/// One table of classes, the largest gain first.
fn print_table(title: &str, table: &BTreeMap<String, Class>) {
    println!("\n{title:<34} questions    gain  per question  repaired");
    let mut rows: Vec<_> = table.iter().collect();
    rows.sort_by(|a, b| b.1.gain.total_cmp(&a.1.gain));
    for (name, class) in rows {
        #[allow(clippy::cast_precision_loss)]
        let each = class.gain / class.questions.max(1) as f64;
        println!(
            "{name:<34} {:>9} {:>7.2} {each:>13.3} {:>9}",
            class.questions, class.gain, class.repaired
        );
    }
}
