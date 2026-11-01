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
