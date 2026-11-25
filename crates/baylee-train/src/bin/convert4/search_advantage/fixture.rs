//! A short game, whose seat 0 plays lands and casts creatures for a few turns
//! before its opponent gives up, and the search events a test gives its inputs.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};

use baylee_core::ids::PlayerId;
use baylee_core::preset::{AIProfile, DeckEntry, GamePreset};
use baylee_engine::choice::{Pending, PlayerAction};
use baylee_gamehost::Session;
use baylee_train::convert4::search_roots::Roots4;
use baylee_train::convert4::{Asks4, Step4, TeamHands, stream_game4_asking};
use baylee_train::housedeck::HouseDeck;
use baylee_train::selfplay::table_for;
use serde_json::{Value, json};

/// The seat whose inputs are roots.
pub(crate) const SEAT: u8 = 0;
const ME: PlayerId = PlayerId::new(SEAT);
const THEM: PlayerId = PlayerId::new(1);

/// Two candidates' scores in four worlds, the second ahead of the first by
/// 0.1 with a standard error of [`AHEAD_SE`].
pub(crate) const WORLDS: [[f64; 2]; 4] = [[0.5, 0.6], [0.5, 0.4], [0.5, 0.9], [0.5, 0.5]];
/// The standard error of the second candidate's lead in [`WORLDS`].
pub(crate) const AHEAD_SE: f64 = 0.108_012;

/// The game.
pub(crate) struct Recorded {
    pub(crate) record: Vec<u8>,
    /// The seat's tap, land and cast inputs: number and answer.
    pub(crate) acts: Vec<(u64, PlayerAction)>,
    pub(crate) rule_roots: Arc<BTreeMap<String, u32>>,
}

pub(crate) fn recorded() -> &'static Recorded {
    static GAME: OnceLock<Recorded> = OnceLock::new();
    GAME.get_or_init(|| {
        let preset = preset();
        let record = play(&preset);
        let acts = super::record_inputs(&record)
            .unwrap()
            .into_iter()
            .filter(|(seat, _, action)| {
                *seat == SEAT && super::root_kind(action).is_some_and(|kind| kind != 0)
            })
            .map(|(_, n, action)| (n, action))
            .collect();
        let cards: BTreeSet<u32> = preset
            .seats
            .iter()
            .flat_map(|seat| {
                seat.deck
                    .iter()
                    .chain(&seat.sideboard)
                    .chain(&seat.commanders)
                    .chain(seat.starting_hand.iter().flatten())
                    .chain(&seat.starting_battlefield)
            })
            .map(|entry| entry.card.get())
            .collect();
        let cards: Vec<u32> = cards.into_iter().collect();
        Recorded {
            record,
            acts,
            rule_roots: Arc::new(baylee_train::cardstructure::catalog(&cards).unwrap().roots),
        }
    })
}

/// Seat 0 with Mountains in play and in hand and creatures to cast off them.
fn preset() -> GamePreset {
    let entry = |name: &&str| DeckEntry {
        card: baylee_cards::all()
            .find(|card| card.name() == *name)
            .unwrap_or_else(|| panic!("{name} is in the pool"))
            .index,
        print: baylee_core::ids::PrintRef::new(0),
        rule_print: None,
        choices: baylee_core::card_metadata::DeckChoices::default(),
    };
    let deck = HouseDeck::named("victory").unwrap();
    let mut preset = table_for(12, &[&deck, &deck], &[AIProfile::STEADY; 2], &[]);
    let hand = ["Mountain", "Mountain", "Mountain"]
        .into_iter()
        .chain(["Mons's Goblin Raiders"; 4]);
    preset.seats[0].starting_hand = Some(hand.map(|name| entry(&name)).collect());
    preset.seats[0].starting_battlefield = ["Mountain"; 2].iter().map(entry).collect();
    preset
}

/// Four rounds of it, to the opponent's giving up.
fn play(preset: &GamePreset) -> Vec<u8> {
    let mut session = Session::new_recorded(preset, "t-search-advantage").unwrap();
    for seat in [ME, THEM] {
        session.take_over(seat);
    }
    loop {
        let seat = session.pending().asked().unwrap();
        if seat == THEM && session.observe_view(ME).turn >= 8 {
            break;
        }
        let action = match session.pending() {
            Pending::Mulligan { .. } => PlayerAction::MulliganKeep,
            Pending::Priority { legal, .. } => {
                let view = session.observe_view(ME);
                let mine = seat == ME
                    && view.active == ME
                    && view.phase == baylee_view::Phase::FirstMain
                    && view.stack.is_empty();
                if !mine {
                    PlayerAction::PassPriority
                } else if let Some(&card) = legal.castable.first() {
                    PlayerAction::CastSpell { card }
                } else if let Some(&card) = legal.lands.first() {
                    PlayerAction::PlayLand { card }
                } else if let (Some(&source), false) =
                    (legal.mana_abilities.first(), legal.payable.is_empty())
                {
                    PlayerAction::ActivateManaAbility { source }
                } else {
                    PlayerAction::PassPriority
                }
            }
            _ => session.house_action(seat).unwrap(),
        };
        session.act(seat, action).unwrap();
    }
    session.act(THEM, PlayerAction::Concede).unwrap();
    assert!(matches!(session.pending(), Pending::GameOver(_)));
    session.take_record()
}

/// The event of a root answered with `action` at input `n`, whose search tried
/// that answer and a pass.
pub(crate) fn event(n: u64, action: &PlayerAction) -> Value {
    json!({"accepted": true, "seat": SEAT, "record_input_n": n, "action": action,
        "candidates": [action, "PassPriority"], "scripts": [null, null], "worlds": WORLDS,
        "chosen": 1, "leaves": {"stopped": 0}, "diffs": [0.0, 0.1], "ses": [0.0, AHEAD_SE]})
}

/// A `games.jsonl` entry with `events`.
pub(crate) fn game(events: &[Value]) -> Value {
    json!({"fallback_answers": 0, "refused_answers": 0, "search_events": events})
}

/// Streams the game, asking for `roots`.
pub(crate) fn stream(
    roots: &Roots4,
    emit: impl FnMut(Step4) -> Result<(), String>,
) -> Result<(), String> {
    stream_game4_asking(
        &recorded().record,
        4,
        true,
        None,
        None,
        None,
        false,
        10,
        None,
        TeamHands::Private,
        Asks4 {
            roots: Some(roots),
            ..Asks4::default()
        },
        emit,
    )
    .map(|_| ())
}

/// A directory of a test's own, removed with it.
pub(crate) struct Directory(pub(crate) PathBuf);

impl Directory {
    pub(crate) fn new() -> Self {
        let path = std::env::temp_dir().join(format!("baylee-search-{}", uuid::Uuid::now_v7()));
        std::fs::create_dir_all(&path).unwrap();
        Self(path)
    }

    pub(crate) fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for Directory {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
