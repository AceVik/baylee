//! Reach on positions a real engine dealt.
//!
//! The rest of this file's siblings build the view and the offer by hand,
//! which proves the orders read them right and nothing about what the engine
//! puts in them. What a cast costs under a reducer and whether a spell has
//! anything to point at are the engine's to say (`LegalActions`), so these
//! tables are dealt by a `Session` and read exactly as the bridge reads one.

use baylee_core::ids::PlayerId;
use baylee_core::preset::{DeckEntry, GamePreset, SeatController};
use baylee_engine::choice::{LegalActions, Pending, PlayerAction};
use baylee_gamehost::Session;
use baylee_view::{Phase, PlayerView};

use super::super::{Reach, reachable};

const ME: PlayerId = PlayerId::new(0);

fn entry(name: &str) -> DeckEntry {
    DeckEntry {
        card: baylee_cards::decks::by_name(name)
            .unwrap_or_else(|| panic!("{name} is not in the pool")),
        print: baylee_core::ids::PrintRef::new(0),
        rule_print: None,
        choices: baylee_core::card_metadata::DeckChoices::default(),
    }
}

/// Seat 0's first main phase with `mine` on its battlefield and `hand` in its
/// hand, `theirs` across the table, and the view and offer it is asked with.
pub(super) fn position(
    mine: &[&str],
    hand: &[&str],
    theirs: &[&str],
) -> (Session, PlayerView, LegalActions) {
    let mut preset: GamePreset =
        baylee_cards::decks::probe_preset(41, entry("Mountain").card).unwrap();
    preset.house_rules.starting_player = Some(0);
    for seat in &mut preset.seats {
        seat.controller = SeatController::Human { user_id: 0 };
        seat.starting_hand = Some(Vec::new());
        seat.starting_battlefield.clear();
    }
    preset.seats[0].starting_hand = Some(hand.iter().map(|name| entry(name)).collect());
    preset.seats[0].starting_battlefield = mine.iter().map(|name| entry(name)).collect();
    preset.seats[1].starting_battlefield = theirs.iter().map(|name| entry(name)).collect();
    let mut session = Session::new(&preset).unwrap();
    for _ in 0..100 {
        let pending = session.pending().clone();
        let player = pending.asked().expect("the table stays playable");
        if let Pending::Priority { player, .. } = pending
            && player == ME
            && session.observe_view(ME).phase == Phase::FirstMain
        {
            let (pending, view) = session.view_for(ME).expect("seat 0 is asked");
            let Pending::Priority { legal, .. } = pending else {
                unreachable!("seat 0 holds priority");
            };
            return (session, view, *legal);
        }
        let action = match pending {
            Pending::Mulligan { .. } => PlayerAction::MulliganKeep,
            Pending::Priority { .. } => PlayerAction::PassPriority,
            other => panic!("unexpected setup question: {other:?}"),
        };
        session.act_slice(player, action, 0).unwrap();
    }
    panic!("seat 0's first main phase was not reached");
}

/// The reach for the card named `name`, if the orders found one.
pub(super) fn reach_of<'a>(reach: &'a [Reach], name: &str) -> Option<&'a Reach> {
    let card = entry(name).card;
    reach.iter().find(|r| r.card.index == card)
}

/// Goblin Warchief: "Goblin spells you cast cost {1} less to cast." The
/// engine charges the Sky Raider `{1}{R}` under it, and the seat priced the
/// printed `{2}{R}`: with two Mountains the Goblin was never offered, and with
/// three it was offered tapping all three, leaving a mana floating that
/// nothing could spend (NEW-03, 3.11.2026; CR 601.2f).
#[test]
fn a_goblin_under_the_warchief_is_reached_for_its_reduced_price() {
    let (_, view, legal) = position(
        &["Goblin Warchief", "Mountain", "Mountain"],
        &["Goblin Sky Raider"],
        &[],
    );
    let reach = reachable(&view, &legal);
    let raider = reach_of(&reach, "Goblin Sky Raider")
        .expect("two Mountains pay the Sky Raider's {1}{R} under the Warchief");
    assert_eq!(raider.plan.steps.len(), 2);

    let (_, view, legal) = position(
        &["Goblin Warchief", "Mountain", "Mountain", "Mountain"],
        &["Goblin Sky Raider"],
        &[],
    );
    let reach = reachable(&view, &legal);
    let raider = reach_of(&reach, "Goblin Sky Raider").expect("three Mountains pay it");
    assert_eq!(
        raider.plan.steps.len(),
        2,
        "the third Mountain is not tapped for a price the engine does not charge"
    );
}

/// A spell that must target and has no legal target cannot be cast
/// (CR 601.2c), and the orders tapped the lands for it anyway: Terror over
/// empty battlefields, Terror at a black creature, Fear (an Aura targets
/// what it will enchant, CR 303.4a) with no creature, Disenchant over a board
/// of creatures, Righteousness with nothing blocking. The engine refused the
/// cast with the mana already floating (NEW-04, 3.11.2026). What a spell may
/// target is the engine's to say: it lists the card in neither `castable`
/// nor `payable`.
#[test]
fn a_spell_with_no_legal_target_is_not_reached_for() {
    for (spell, lands, theirs) in [
        ("Terror", &["Swamp", "Swamp"][..], &[][..]),
        ("Terror", &["Swamp", "Swamp"][..], &["Vampire Bats"][..]),
        ("Fear", &["Swamp", "Swamp"][..], &[][..]),
        (
            "Disenchant",
            &["Plains", "Plains"][..],
            &["Grizzly Bears"][..],
        ),
        // "Target blocking creature": in a main phase nothing blocks.
        ("Righteousness", &["Plains"][..], &["Grizzly Bears"][..]),
    ] {
        let (_, view, legal) = position(lands, &[spell], theirs);
        assert!(
            reach_of(&reachable(&view, &legal), spell).is_none(),
            "{spell} reached for with nothing to target among {theirs:?}"
        );
    }
    // And with a target it is reached for: the engine's answer, not a blanket no.
    let (_, view, legal) = position(&["Swamp", "Swamp"], &["Terror"], &["Grizzly Bears"]);
    let reach = reachable(&view, &legal);
    let terror = reach_of(&reach, "Terror").expect("the Bears are a target");
    assert_eq!(terror.plan.steps.len(), 2);
}
