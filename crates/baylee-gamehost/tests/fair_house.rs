//! A hosted house plays from what its seat may see (owner, 10.11.2026).
//!
//! The gateway and the engine server host a game through `Session::new` and
//! never call `set_fair_scouting`, so this is the table a player meets. Two
//! deals are told apart only by what the house may not see: the opponent's
//! decklist, hand and library. The opponent keeps, starts with no cards and
//! only ever passes, so the public table stays the same in both for turns.
//! Every fair level answers both deals identically, answer for answer; the
//! level that peeks does not, which is what shows the deals differ where a
//! peek would see it.

use baylee_cards::decks::{LoadedDeck, by_name, house_deck, preset_for, probe_deck};
use baylee_core::ids::PlayerId;
use baylee_core::preset::{AIProfile, GamePreset, SeatController};
use baylee_engine::choice::{Pending, PlayerAction, timeout_answer};
use baylee_gamehost::Session;
use baylee_gamehost::record::{Line, Source};

/// How far each deal is played: the opponent has drawn a handful of cards
/// and discarded none, so nothing of theirs has been shown.
const LAST_TURN: u32 = 10;

/// One answer of the house and the public table after it.
type Answer = (PlayerAction, String);

fn house(name: &str) -> LoadedDeck {
    let path = format!("{}/../../data/decks/{name}.txt", env!("CARGO_MANIFEST_DIR"));
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{path}: {e}"));
    house_deck(&text).unwrap_or_else(|e| panic!("{name}: {e}"))
}

/// A plain deck where holding mana up is a decision every turn: two-drops
/// and a counterspell, which waits for something to counter.
fn merfolk_and_counters() -> LoadedDeck {
    house_deck("[deck:Merfolk]\n24 Island\n20 Coral Merfolk\n16 Counterspell\n")
        .expect("the pool has all three")
}

/// Sixty cards, `copies` of `card` and basics of its colour for the rest.
fn opponent(card: &str, copies: usize) -> LoadedDeck {
    let index = by_name(card).unwrap_or_else(|| panic!("the pool has {card}"));
    probe_deck(index, copies, 60).expect("basics to pad with")
}

/// The house at `profile` in seat 0 against a person in seat 1 who holds no
/// cards at the start, as a hosted room seats them.
fn deal(seed: u64, house_deck: &LoadedDeck, theirs: &LoadedDeck, profile: AIProfile) -> GamePreset {
    let mut preset = preset_for(seed, house_deck, theirs);
    preset.house_rules.starting_player = Some(0);
    preset.seats[0].controller = SeatController::Ai(profile);
    preset.seats[1].controller = SeatController::Open;
    preset.seats[1].starting_hand = Some(Vec::new());
    preset
}

/// Every answer the house gave, with the public table after it, until
/// [`LAST_TURN`] or the end.
fn house_answers(preset: &GamePreset) -> Vec<Answer> {
    let mut session = Session::new_recorded(preset, "fair-house").expect("the table starts");
    session.describe("fair-house".into(), vec!["house".into(), "person".into()]);
    let person = PlayerId::new(1);
    for _ in 0..10_000 {
        let _ = session.pump();
        if matches!(session.pending(), Pending::GameOver(_))
            || session.state().turn.number > LAST_TURN
        {
            break;
        }
        let Some((pending, _)) = session.view_for(person) else {
            continue;
        };
        let nothing = timeout_answer(&pending).unwrap_or(PlayerAction::PassPriority);
        session
            .act(person, nothing)
            .expect("doing nothing is an answer");
    }
    let record = String::from_utf8(session.take_record()).expect("a record is text");
    record
        .lines()
        .filter_map(|line| match serde_json::from_str::<Line>(line).ok()? {
            Line::Input {
                by: Source::House,
                action,
                seen,
                ..
            } => Some((action, seen.unwrap_or_default())),
            _ => None,
        })
        .collect()
}

/// The two deals: an opponent whose deck and draws are all Plains, and one
/// whose deck is half Llanowar Elves. Both hold no cards at the start.
fn deals(seed: u64, house_deck: &LoadedDeck, profile: AIProfile) -> [Vec<Answer>; 2] {
    [
        house_answers(&deal(seed, house_deck, &opponent("Plains", 60), profile)),
        house_answers(&deal(
            seed,
            house_deck,
            &opponent("Llanowar Elves", 30),
            profile,
        )),
    ]
}

/// The seeds every deck is dealt with: the control below finds a peek in
/// some of them and not in others, so a fair level is held to all of them.
const SEEDS: std::ops::Range<u64> = 11..21;

#[test]
fn a_hosted_fair_house_answers_the_same_whatever_is_hidden() {
    let merfolk = merfolk_and_counters();
    let allytifact = house("allytifact");
    let weltenbaum = house("weltenbaum");
    let tables = SEEDS
        .map(|seed| (seed, &merfolk))
        .chain([(11, &allytifact), (11, &weltenbaum)]);
    let tables: Vec<_> = tables.collect();
    for (name, profile) in AIProfile::NAMED.iter().chain(&AIProfile::REFERENCES) {
        if profile.peeks() {
            continue;
        }
        for (seed, deck) in &tables {
            let [plains, elves] = deals(*seed, deck, *profile);
            assert!(plains.len() > 20, "{name}: the deal asked the house little");
            if let Some(n) = plains
                .iter()
                .zip(&elves)
                .position(|((a, _), (b, _))| a != b)
            {
                panic!(
                    "{name} answered differently at answer {n} when only hidden cards \
                     differed: {:?} against {:?}",
                    plains[n].0, elves[n].0
                );
            }
            assert_eq!(plains, elves, "{name}: the answers or the tables differ");
        }
    }
}

/// The control: the same two deals do differ for a house that peeks, so
/// the test above would see a fair level that peeked. Merfolk and
/// Counterspell make the difference reachable: a house that holds a cheap
/// instant beside a creature keeps its mana up while an opponent may have
/// something to answer (`HoldUp`), and one that has looked and found only
/// Plains spends it on the next Merfolk instead.
#[test]
fn the_level_that_peeks_answers_the_two_deals_differently() {
    let actions = |answers: &[Answer]| -> Vec<PlayerAction> {
        answers.iter().map(|(a, _)| a.clone()).collect()
    };
    let differs = SEEDS.filter(|seed| {
        let [plains, elves] = deals(*seed, &merfolk_and_counters(), AIProfile::SEER);
        actions(&plains) != actions(&elves)
    });
    assert!(
        differs.count() > 0,
        "seer never read the hidden cards it is shown"
    );
}
