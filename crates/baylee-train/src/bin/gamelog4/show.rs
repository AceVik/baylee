//! Prints one game turn by turn.

use std::fmt::Write as _;
use std::path::Path;

use baylee_core::ids::PlayerId;
use baylee_engine::choice::{LegalActions, Pending, PlayerAction};

use crate::board;
use crate::game::{Event, Game};
use crate::names::{self, Names};

/// Which part of a game to print.
pub(super) struct Shown {
    pub from_turn: u32,
    pub to_turn: u32,
    pub forced: bool,
}

impl Shown {
    fn turn(&self, turn: u32) -> bool {
        (self.from_turn..=self.to_turn).contains(&turn)
    }
}

/// What else a seat that passed priority could have done.
pub(super) fn alternatives(legal: &LegalActions, names: &Names) -> Vec<String> {
    let mut options: Vec<String> = Vec::new();
    options.extend(
        legal
            .lands
            .iter()
            .map(|id| format!("play {}", names.name(*id))),
    );
    options.extend(
        legal
            .castable
            .iter()
            .map(|id| format!("cast {}", names.name(*id))),
    );
    // Castable while paid for: whether the seat's sources make that mana is
    // not checked (`LegalActions::payable`).
    options.extend(
        legal
            .payable
            .iter()
            .map(|id| format!("cast? {}", names.name(*id))),
    );
    options.extend(
        legal
            .abilities
            .iter()
            .map(|(id, _)| format!("activate {}", names.name(*id))),
    );
    options.extend(
        legal
            .foretellable
            .iter()
            .map(|id| format!("foretell {}", names.name(*id))),
    );
    options.extend(
        legal
            .suspendable
            .iter()
            .map(|id| format!("suspend {}", names.name(*id))),
    );
    options.dedup();
    options
}

/// The question's kind, as its `Debug` names it.
pub(super) fn kind(pending: &Pending) -> String {
    let question = format!("{pending:?}");
    question
        .split([' ', '{', '('])
        .next()
        .unwrap_or("?")
        .to_owned()
}

/// The question asked and the answer given, with names.
fn describe(pending: &Pending, action: &PlayerAction, names: &Names) -> String {
    let mut text = format!(
        "{}: {}",
        kind(pending),
        names.annotate(&format!("{action:?}"))
    );
    if let (Pending::Priority { legal, .. }, PlayerAction::PassPriority) = (pending, action) {
        let options = alternatives(legal, names);
        if !options.is_empty() {
            let _ = write!(text, "   (could: {})", options.join(", "));
        }
    }
    text
}

/// Prints game `game` of the eval4 record directory `run`.
pub(super) fn show(run: &Path, game: u64, shown: &Shown) -> anyhow::Result<()> {
    let record = Game::load(run, game)?;
    println!(
        "{} · deal {game} · seed {} · the net is {} · outcome {}",
        record.row["game"].as_str().unwrap_or("?"),
        record.row["seed"],
        names::seat(record.seat),
        record.row["outcome"]
    );
    let focal = usize::from(record.seat.get());
    record.replay(true, |event| {
        match event {
            Event::Turn { views, turn } if shown.turn(turn) => board::print(views, focal, turn),
            Event::Decision(decision)
                if shown.turn(decision.views[focal].turn)
                    && (shown.forced || decision.source != "Forced") =>
            {
                let names = Names::of(decision.views);
                println!(
                    "  » {:<6} {}",
                    decision.source,
                    describe(decision.pending, decision.action, &names)
                );
            }
            Event::Log { turn, text } if shown.turn(turn) => println!("      {text}"),
            Event::End {
                winners,
                reason,
                turn,
            } => println!(
                "\n=== Over in turn {}: won by {} ({reason})",
                turn.map_or_else(|| "?".to_owned(), |turn| turn.to_string()),
                winners
                    .iter()
                    .map(|winner| names::seat(PlayerId::new(*winner)))
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            _ => {}
        }
        Ok(())
    })
}
