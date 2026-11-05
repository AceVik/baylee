//! Foretell (CR 702.143a): the permission to cast a card for its foretell
//! cost belongs to the card exiled face down by the special action, for as
//! long as it stays exiled, and to no later object (CR 400.7).
//!
//! Played with Doomskar ({3}{W}{W}, foretell {1}{W}{W}).

use super::testkit::{
    Duel, RegistryLookup, at_rest, card_index, keep_mulligans, pass_until, reach_main_phase,
    tap_all_mana,
};
use super::*;
use crate::event::Cause;
use crate::object::Rider;
use crate::zone::{Zone, ZoneLocation, ZonePosition};
use baylee_core::ids::{CardIndex, ObjectId};

const SEED: u64 = 0x0f07_e11e;

fn doomskar() -> CardIndex {
    card_index("38ece0fa-2c94-4eae-b6ab-74b783862f3e")
}
fn plains() -> CardIndex {
    card_index("bc71ebf6-2056-41f7-be35-b2e5c34afa99")
}
fn forest() -> CardIndex {
    card_index("b34bb2dc-c1af-4d77-b0b3-a0fb342a5fc6")
}

/// Doomskar foretold on turn one, cast for its foretell cost on its owner's
/// next turn, and left on the stack.
fn foretold_doomskar_on_the_stack() -> (Engine<RegistryLookup>, ObjectId) {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[plains(); 5])
        .hand(0, &[doomskar()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let card = engine.state.zones.list(ZoneLocation::Hand(p0))[0];
    tap_all_mana(&mut engine, p0);
    engine
        .apply(p0, PlayerAction::Foretell { card })
        .expect("foretell pays {2}");
    pass_until(&mut engine, |e| {
        e.state().turn.number > 2
            && e.state().turn.active == p0
            && matches!(e.state().turn.phase, Phase::FirstMain)
            && at_rest(e, p0)
    });
    assert!(crate::casting::foretell_permission(
        engine.state(),
        p0,
        card
    ));
    tap_all_mana(&mut engine, p0);
    engine
        .apply(p0, PlayerAction::CastSpell { card })
        .expect("a later turn offers the foretell cost");
    if let Pending::ChooseCastMode { options, .. } = engine.pending().clone() {
        let foretell = options
            .iter()
            .position(|option| option.kind == crate::choice::CastModeKind::Foretell)
            .expect("the foretell cost is offered");
        engine
            .apply(p0, PlayerAction::ChooseMode(foretell))
            .unwrap();
    }
    assert_eq!(engine.state().object(card).unwrap().zone, Zone::Stack);
    (engine, card)
}

/// A foretold spell exiled from the stack (a counterspell that exiles, say)
/// is a new object in exile: not foretold, so not castable for {1}{W}{W}.
/// The rider survived every move into exile, and the card was castable for
/// its foretell cost again and again.
#[test]
fn a_foretold_spell_exiled_from_the_stack_is_no_longer_foretold() {
    let (mut engine, card) = foretold_doomskar_on_the_stack();
    let p0 = PlayerId::new(0);
    // On the stack the spell answers "was foretold" (Starnheim Unleashed's X).
    assert!(
        engine
            .state()
            .object(card)
            .unwrap()
            .riders
            .iter()
            .any(|r| matches!(r, Rider::Foretold { .. }))
    );
    engine
        .state
        .move_object(
            card,
            ZoneLocation::Exile(p0),
            ZonePosition::Top,
            Cause::Effect,
        )
        .unwrap();
    let exiled = engine.state().object(card).unwrap();
    assert_eq!(exiled.zone, Zone::Exile);
    assert!(
        !exiled
            .riders
            .iter()
            .any(|r| matches!(r, Rider::Foretold { .. })),
        "{:?}",
        exiled.riders
    );
    assert!(!crate::casting::foretell_permission(
        engine.state(),
        p0,
        card
    ));
    assert!(crate::casting::can_cast(engine.state(), &RegistryLookup, p0, card).is_err());
}

/// The control: a foretold card that moves from exile to exile is still the
/// card in exile, and keeps the permission.
#[test]
fn a_foretold_card_moved_within_exile_stays_foretold() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[plains(); 5])
        .hand(0, &[doomskar()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let card = engine.state.zones.list(ZoneLocation::Hand(p0))[0];
    tap_all_mana(&mut engine, p0);
    engine.apply(p0, PlayerAction::Foretell { card }).unwrap();
    engine
        .state
        .move_object(
            card,
            ZoneLocation::Exile(p0),
            ZonePosition::Top,
            Cause::Effect,
        )
        .unwrap();
    assert!(
        engine
            .state()
            .object(card)
            .unwrap()
            .riders
            .iter()
            .any(|r| matches!(r, Rider::Foretold { .. }))
    );
}

/// Bug hunt of 3.11.2026, game 1, q65: Starnheim Unleashed was never
/// offered for foretell with two untapped lands, because the {2} had to be
/// floating already. The engine names such a card a planning hint
/// (`unpaid_foretell`), a seat taps for the {2} (CR 605.3a), and the card
/// is then `foretellable` (CR 702.143a).
#[test]
fn a_card_whose_foretell_the_lands_can_pay_is_a_hint_until_the_mana_is_made() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[plains(), plains()])
        .hand(0, &[doomskar()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let card = engine.state.zones.list(ZoneLocation::Hand(p0))[0];
    let legal = engine.compute_legal(p0);
    assert_eq!(legal.unpaid_foretell, vec![card]);
    assert!(legal.foretellable.is_empty());
    assert!(
        engine.apply(p0, PlayerAction::Foretell { card }).is_err(),
        "a hint is not an action"
    );
    assert_eq!(tap_all_mana(&mut engine, p0), 2);
    let legal = engine.compute_legal(p0);
    assert_eq!(legal.foretellable, vec![card]);
    assert!(legal.unpaid_foretell.is_empty());
    engine
        .apply(p0, PlayerAction::Foretell { card })
        .expect("the {2} is floating now");
    assert_eq!(
        engine.state().object(card).map(|o| o.zone),
        Some(Zone::Exile)
    );
}

/// With nothing left to make mana with, foretell is not hinted at.
#[test]
fn no_foretell_hint_without_a_mana_ability_on_offer() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest()).hand(0, &[doomskar()]).start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let legal = engine.compute_legal(p0);
    assert!(legal.unpaid_foretell.is_empty());
    assert!(legal.foretellable.is_empty());
}
