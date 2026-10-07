//! An enters-the-battlefield ability triggers whenever its permanent enters,
//! from whatever zone (CR 603.6a): cast from a hand, returned from a
//! graveyard, from exile, put onto the battlefield from a library.
//!
//! Played with the pool's reanimators on Merchant of Secrets ("When this
//! creature enters, draw a card") and Mulldrifter ("… draw two cards"),
//! whose triggers are counted in the hand they fill.

use super::testkit::{
    Duel, RegistryLookup, card_index, cast_from_hand, keep_mulligans, pass_until, stack_is_empty,
    walk_to_own_main,
};
use super::*;
use crate::event::Cause;
use crate::zone::{ZoneLocation, ZonePosition};
use baylee_core::ids::{CardIndex, ObjectId};

fn swamp() -> CardIndex {
    card_index("56719f6a-1a6c-4c0a-8d21-18f7d7350b68")
}
fn island() -> CardIndex {
    card_index("b2c6aa39-2d2a-459c-a555-fb48ba993373")
}
fn plains() -> CardIndex {
    card_index("bc71ebf6-2056-41f7-be35-b2e5c34afa99")
}
fn merchant_of_secrets() -> CardIndex {
    card_index("f6aebd42-0150-4741-84c2-4c85893640e9")
}
fn mulldrifter() -> CardIndex {
    card_index("24d0f5e7-0d9e-4b76-900e-a7274e80312d")
}
fn reanimate() -> CardIndex {
    card_index("a044474a-cd72-4e9d-bd8d-a08f2de9cdc0")
}
fn animate_dead() -> CardIndex {
    card_index("c0d8fef4-65f4-4769-982d-b397d2b7e977")
}
fn karmic_guide() -> CardIndex {
    card_index("8c31fec9-e4b3-4761-990e-7be38eb05604")
}
fn sun_titan() -> CardIndex {
    card_index("b2e950fb-cb7e-40a0-a311-5bbdd0477b29")
}
fn ephemerate() -> CardIndex {
    card_index("0fd57894-b917-41c8-a394-360d1d31b236")
}

const P0: PlayerId = PlayerId::new(0);

fn hand_size(engine: &Engine<RegistryLookup>) -> usize {
    engine.state().zones.list(ZoneLocation::Hand(P0)).len()
}

/// Seat 0's copy of `card` in its hand.
fn in_hand(engine: &Engine<RegistryLookup>, card: CardIndex) -> ObjectId {
    *engine
        .state()
        .zones
        .list(ZoneLocation::Hand(P0))
        .iter()
        .find(|id| {
            engine
                .state()
                .object(**id)
                .and_then(|o| o.card)
                .is_some_and(|c| c.index == card)
        })
        .expect("the card is in hand")
}

/// Seat 0 at its first main phase with `hand` in hand on `lands`, and
/// `dead` already in its graveyard.
fn a_table(
    hand: &[CardIndex],
    lands: &[CardIndex],
    dead: CardIndex,
) -> (Engine<RegistryLookup>, ObjectId) {
    let mut all = hand.to_vec();
    all.push(dead);
    let mut engine = Duel::new(3, island())
        .hand(0, &all)
        .battlefield(0, lands)
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, P0), "seat 0 reaches its main");
    let card = in_hand(&engine, dead);
    engine
        .dev_state_mut(P0)
        .expect("dev commands")
        .move_object(
            card,
            ZoneLocation::Graveyard(P0),
            ZonePosition::Top,
            Cause::Effect,
        )
        .expect("binned");
    (engine, card)
}

/// Answers the first target question with `target`, then lets the game run
/// until the stack is empty.
fn aim_and_settle(engine: &mut Engine<RegistryLookup>, target: ObjectId) {
    pass_until(engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    engine
        .apply(
            P0,
            PlayerAction::ChooseTargets {
                objects: vec![target],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(engine, stack_is_empty);
}

fn on_battlefield(engine: &Engine<RegistryLookup>, id: ObjectId) -> bool {
    engine
        .state()
        .zones
        .list(ZoneLocation::Battlefield)
        .contains(&id)
}

/// Reanimate returns Mulldrifter from the graveyard; it draws two.
#[test]
fn reanimate_returns_a_creature_whose_enters_trigger_fires() {
    let (mut engine, dead) = a_table(&[reanimate()], &[swamp()], mulldrifter());
    let before = hand_size(&engine);
    cast_from_hand(&mut engine, P0, reanimate());
    aim_and_settle(&mut engine, dead);
    assert!(on_battlefield(&engine, dead), "Mulldrifter is back");
    assert_eq!(
        hand_size(&engine),
        before - 1 + 2,
        "the spell left the hand and the ETB drew two"
    );
}

/// Animate Dead returns Merchant of Secrets; it draws one.
#[test]
fn animate_dead_returns_a_creature_whose_enters_trigger_fires() {
    let (mut engine, dead) = a_table(
        &[animate_dead()],
        &[swamp(), swamp()],
        merchant_of_secrets(),
    );
    let before = hand_size(&engine);
    cast_from_hand(&mut engine, P0, animate_dead());
    aim_and_settle(&mut engine, dead);
    assert!(on_battlefield(&engine, dead), "Merchant is back");
    assert_eq!(
        hand_size(&engine),
        before - 1 + 1,
        "the spell left the hand and the ETB drew one"
    );
}

/// Karmic Guide's own enter trigger returns Merchant of Secrets, whose enter
/// trigger then fires in turn.
#[test]
fn karmic_guide_returns_a_creature_whose_enters_trigger_fires() {
    let (mut engine, dead) = a_table(&[karmic_guide()], &[plains(); 5], merchant_of_secrets());
    let before = hand_size(&engine);
    cast_from_hand(&mut engine, P0, karmic_guide());
    aim_and_settle(&mut engine, dead);
    assert!(on_battlefield(&engine, dead), "Merchant is back");
    assert_eq!(
        hand_size(&engine),
        before - 1 + 1,
        "the spell left the hand and the ETB drew one"
    );
}

/// Sun Titan's enter trigger returns Merchant of Secrets ("you may", which
/// the pass answers yes); Merchant draws one.
#[test]
fn sun_titan_returns_a_permanent_whose_enters_trigger_fires() {
    let (mut engine, dead) = a_table(&[sun_titan()], &[plains(); 6], merchant_of_secrets());
    let before = hand_size(&engine);
    cast_from_hand(&mut engine, P0, sun_titan());
    aim_and_settle(&mut engine, dead);
    assert!(on_battlefield(&engine, dead), "Merchant is back");
    assert_eq!(
        hand_size(&engine),
        before - 1 + 1,
        "the spell left the hand and the ETB drew one"
    );
}

/// Ephemerate exiles Merchant of Secrets and returns it from exile: it draws
/// one as it comes back.
#[test]
fn a_creature_returned_from_exile_triggers_as_it_enters() {
    let mut engine = Duel::new(3, island())
        .hand(0, &[ephemerate()])
        .battlefield(0, &[plains(), merchant_of_secrets()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, P0), "seat 0 reaches its main");
    let merchant = *engine
        .state()
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .find(|id| {
            engine
                .state()
                .object(**id)
                .and_then(|o| o.card)
                .is_some_and(|c| c.index == merchant_of_secrets())
        })
        .expect("Merchant is on the battlefield");
    let before = hand_size(&engine);
    cast_from_hand(&mut engine, P0, ephemerate());
    aim_and_settle(&mut engine, merchant);
    assert!(on_battlefield(&engine, merchant), "Merchant is back");
    assert_eq!(
        hand_size(&engine),
        before - 1 + 1,
        "Ephemerate left the hand and the returned Merchant drew one"
    );
}

fn werefox_bodyguard() -> CardIndex {
    card_index("d5ee2ced-29f4-430f-962e-2f930b92624c")
}

/// Werefox Bodyguard exiles Merchant of Secrets "until this creature leaves
/// the battlefield"; sacrificing it returns the Merchant (CR 610.3). That
/// return is made inside the move that takes the Bodyguard away, the one
/// arrival journalled from within another move, and it triggers all the
/// same.
#[test]
fn a_creature_returned_as_its_exiler_leaves_triggers_as_it_enters() {
    let mut engine = Duel::new(3, island())
        .hand(0, &[werefox_bodyguard()])
        .battlefield(
            0,
            &[
                plains(),
                plains(),
                plains(),
                plains(),
                plains(),
                merchant_of_secrets(),
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, P0), "seat 0 reaches its main");
    let merchant = *engine
        .state()
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .find(|id| {
            engine
                .state()
                .object(**id)
                .and_then(|o| o.card)
                .is_some_and(|c| c.index == merchant_of_secrets())
        })
        .expect("Merchant is on the battlefield");
    cast_from_hand(&mut engine, P0, werefox_bodyguard());
    aim_and_settle(&mut engine, merchant);
    assert!(!on_battlefield(&engine, merchant), "the Merchant is exiled");
    let fox =
        super::testkit::on_battlefield(&engine, P0, werefox_bodyguard()).expect("the Bodyguard");
    let before = hand_size(&engine);
    engine
        .apply(
            P0,
            PlayerAction::ActivateAbility {
                source: fox,
                ability_index: 1,
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert!(on_battlefield(&engine, merchant), "the Merchant is back");
    assert_eq!(hand_size(&engine), before + 1, "and its ETB drew one");
}

/// The move itself, from every zone a card can be put onto the battlefield
/// from: the trigger belongs to the arrival, whatever made it.
#[test]
fn an_enters_trigger_fires_from_every_zone() {
    for from in [
        ZoneLocation::Graveyard(P0),
        ZoneLocation::Exile(P0),
        ZoneLocation::Library(P0),
        ZoneLocation::Hand(P0),
    ] {
        let mut engine = Duel::new(3, island())
            .hand(0, &[merchant_of_secrets()])
            .start();
        keep_mulligans(&mut engine);
        assert!(walk_to_own_main(&mut engine, P0), "seat 0 reaches its main");
        let merchant = in_hand(&engine, merchant_of_secrets());
        let state = engine.dev_state_mut(P0).expect("dev commands");
        if from != ZoneLocation::Hand(P0) {
            state
                .move_object(merchant, from, ZonePosition::Top, Cause::Effect)
                .expect("staged");
        }
        state
            .move_object(
                merchant,
                ZoneLocation::Battlefield,
                ZonePosition::Top,
                Cause::Effect,
            )
            .expect("entered");
        let before = hand_size(&engine);
        // The arrival is collected on the next pass through the machine.
        engine.apply(P0, PlayerAction::PassPriority).unwrap();
        pass_until(&mut engine, stack_is_empty);
        assert_eq!(hand_size(&engine), before + 1, "from {from:?}");
    }
}
