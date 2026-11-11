//! What an offer says restricted mana may pay for (CR 106.6):
//! `LegalActions::restricted_taps` and `LegalActions::restricted_floating`.
//!
//! The view says what a land makes and never what its mana may be spent on,
//! so these hold that the engine names it as its own payment reads it: for a
//! mana ability still in the land and for mana already floating, per way of
//! casting (a face being a spell of its own, CR 601.3e), and for the payment
//! a window is open for.

use super::testkit::*;
use super::*;
use crate::choice::{CastModeKind, LegalActions, Spend};
use baylee_core::ids::CardIndex;
use baylee_core::mana::ManaColor;

const P0: PlayerId = PlayerId::new(0);

fn named(name: &str) -> CardIndex {
    baylee_cards::decks::by_name(name).unwrap_or_else(|| panic!("{name} is in the pool"))
}

fn legal(engine: &Engine<RegistryLookup>) -> LegalActions {
    match engine.pending() {
        Pending::Priority { legal, .. } => (**legal).clone(),
        other => panic!("expected priority, got {other:?}"),
    }
}

fn table(board: &[&str], hand: &[&str]) -> Engine<RegistryLookup> {
    let board: Vec<CardIndex> = board.iter().copied().map(named).collect();
    let hand: Vec<CardIndex> = hand.iter().copied().map(named).collect();
    let mut engine = Duel::new(7, named("Mountain"))
        .battlefield(0, &board)
        .hand(0, &hand)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);
    engine
}

fn on_board_all(engine: &Engine<RegistryLookup>, name: &str) -> Vec<ObjectId> {
    let card = named(name);
    engine
        .state()
        .zones
        .list(crate::zone::ZoneLocation::Battlefield)
        .iter()
        .copied()
        .filter(|id| {
            engine
                .state()
                .object(*id)
                .is_some_and(|o| o.controller == P0 && o.card.is_some_and(|c| c.index == card))
        })
        .collect()
}

/// Cavern of Souls played from the hand, naming Elf.
fn cavern_naming_elf(board: &[&str], hand: &[&str]) -> (Engine<RegistryLookup>, ObjectId) {
    let mut hand = hand.to_vec();
    hand.push("Cavern of Souls");
    let mut engine = table(board, &hand);
    let cavern = in_hand(&engine, P0, named("Cavern of Souls")).expect("in hand");
    engine
        .apply(P0, PlayerAction::PlayLand { card: cavern })
        .expect("a land drop");
    engine
        .apply(
            P0,
            PlayerAction::ChooseSubtype(baylee_core::generated::subtypes::creature::ELF),
        )
        .expect("Elf is offered");
    let cavern = on_board_all(&engine, "Cavern of Souls")[0];
    (engine, cavern)
}

/// Ancient Ziggurat: "Spend this mana only to cast a creature spell." Each
/// Ziggurat is named for Grizzly Bears and neither for Lightning Bolt.
#[test]
fn a_creature_only_land_is_named_for_creature_spells_alone() {
    let engine = table(
        &["Ancient Ziggurat", "Ancient Ziggurat"],
        &["Grizzly Bears", "Lightning Bolt"],
    );
    let offer = legal(&engine);
    let bears = in_hand(&engine, P0, named("Grizzly Bears")).expect("in hand");
    let bolt = in_hand(&engine, P0, named("Lightning Bolt")).expect("in hand");
    let ziggurats = on_board_all(&engine, "Ancient Ziggurat");
    assert!(offer.payable.contains(&bears) && offer.payable.contains(&bolt));
    let mut named_for_bears = offer.restricted_taps_for(Spend::Cast(bears, CastModeKind::Normal));
    named_for_bears.sort();
    let mut expected: Vec<(ObjectId, u32)> = ziggurats.iter().map(|id| (*id, 0)).collect();
    expected.sort();
    assert_eq!(named_for_bears, expected);
    assert!(
        offer
            .restricted_taps_for(Spend::Cast(bolt, CastModeKind::Normal))
            .is_empty()
    );
    assert!(offer.restricted_floating.is_empty());
}

/// Cavern of Souls naming Elf: its coloured mana is named for an Elf and
/// not for a Bear, still in the land and floating alike.
#[test]
fn a_chosen_type_land_is_named_for_that_type_alone() {
    let (mut engine, cavern) = cavern_naming_elf(
        &["Forest", "Mountain"],
        &["Elvish Archdruid", "Grizzly Bears"],
    );
    let archdruid = in_hand(&engine, P0, named("Elvish Archdruid")).expect("in hand");
    let bears = in_hand(&engine, P0, named("Grizzly Bears")).expect("in hand");
    let offer = legal(&engine);
    assert_eq!(
        offer.restricted_taps_for(Spend::Cast(archdruid, CastModeKind::Normal)),
        vec![(cavern, 1)]
    );
    assert!(
        offer
            .restricted_taps_for(Spend::Cast(bears, CastModeKind::Normal))
            .is_empty()
    );

    engine
        .apply(
            P0,
            PlayerAction::ActivateAbility {
                source: cavern,
                ability_index: 1,
            },
        )
        .expect("the Cavern's coloured mana");
    if matches!(engine.pending(), Pending::ChooseColor { .. }) {
        engine
            .apply(P0, PlayerAction::ChooseColor(ManaColor::Green))
            .expect("green");
    }
    let offer = legal(&engine);
    let mut green = [0; 6];
    green[ManaColor::Green.index()] = 1;
    assert_eq!(
        offer.restricted_floating_for(Spend::Cast(archdruid, CastModeKind::Normal)),
        green
    );
    assert_eq!(
        offer.restricted_floating_for(Spend::Cast(bears, CastModeKind::Normal)),
        [0; 6]
    );
}

/// The window a payable Elf's cast opens is named too: the Cavern's mana may
/// pay what it owes.
#[test]
fn the_payment_a_window_is_open_for_is_named() {
    let (mut engine, cavern) = cavern_naming_elf(&["Forest", "Mountain"], &["Elvish Archdruid"]);
    let archdruid = in_hand(&engine, P0, named("Elvish Archdruid")).expect("in hand");
    assert!(legal(&engine).payable.contains(&archdruid));
    engine
        .apply(P0, PlayerAction::CastSpell { card: archdruid })
        .expect("cast before its mana");
    assert!(engine.payment_window().is_some(), "a window is open");
    let offer = legal(&engine);
    assert_eq!(offer.restricted_taps_for(Spend::Owed), vec![(cavern, 1)]);
}

/// Brazen Borrower beside a Ziggurat: its creature may be paid with the
/// Ziggurat's mana, Petty Theft, an instant, may not (CR 601.3e).
#[test]
fn a_face_cast_is_asked_as_the_spell_it_is() {
    let mut engine = Duel::new(7, named("Island"))
        .battlefield(
            0,
            &[named("Ancient Ziggurat"), named("Island"), named("Island")],
        )
        .battlefield(1, &[named("Grizzly Bears")])
        .hand(0, &[named("Brazen Borrower")])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);
    let borrower = in_hand(&engine, P0, named("Brazen Borrower")).expect("in hand");
    let ziggurat = on_board_all(&engine, "Ancient Ziggurat")[0];
    let offer = legal(&engine);
    assert!(
        offer
            .cast_ways
            .iter()
            .any(|(id, way)| *id == borrower && way.kind == CastModeKind::Face(1)),
        "Petty Theft has a Bear to return: {:?}",
        offer.cast_ways
    );
    assert_eq!(
        offer.restricted_taps_for(Spend::Cast(borrower, CastModeKind::Normal)),
        vec![(ziggurat, 0)]
    );
    assert!(
        offer
            .restricted_taps_for(Spend::cast(borrower, CastModeKind::Face(1)))
            .is_empty(),
        "{:?}",
        offer.restricted_taps
    );
}

/// A board with no restricted mana states nothing.
#[test]
fn an_offer_without_restricted_mana_names_nothing() {
    let engine = table(&["Forest", "Forest"], &["Grizzly Bears"]);
    let offer = legal(&engine);
    assert!(offer.restricted_taps.is_empty() && offer.restricted_floating.is_empty());
}
