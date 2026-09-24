//! Reflexive triggered abilities (CR 603.12), and the two rules they rest on.
//!
//! "Sacrifice it. When you do, …" creates a triggered ability. Nobody wrote
//! the trigger ahead of time: the resolution creates it, from what that
//! resolution has already done. Three properties follow, and each one is a
//! different way to get the card wrong.
//!
//! - It fires only when the action really happened. A land bounced in
//!   response to its own enter trigger is not sacrificed, so it fetches
//!   nothing.
//! - It is a stack object of its own. Players get priority before it
//!   resolves, and it can be countered like any other ability.
//! - It chooses its target as it is put on the stack, after the action.
//!   Eden's "return another target permanent card" may therefore take a card
//!   that its own mill has just put into the graveyard.
//!
//! The two supporting rules are CR 701.21a, which only lets its controller
//! sacrifice a permanent it controls on the battlefield, and CR 608.2b,
//! which re-checks a target when the ability resolves. A synthetic stack
//! object used to skip that re-check.

use super::testkit::*;
use super::*;
use crate::event::{Cause, GameEvent};
use crate::zone::{Zone, ZoneLocation, ZonePosition};
use baylee_core::ids::{AbilityRef, CardIndex, ObjectId};

const SEED: u64 = 603;

fn forest() -> CardIndex {
    basic_forest()
}
fn swamp() -> CardIndex {
    card_index("56719f6a-1a6c-4c0a-8d21-18f7d7350b68")
}
fn cabaretti_courtyard() -> CardIndex {
    card_index("65424bea-fd53-4f85-9757-0b91a6d40ba4")
}
fn riveteers_overlook() -> CardIndex {
    card_index("5548ff43-e5f6-4a63-8562-a2b1de06d6f5")
}
fn brokers_hideout() -> CardIndex {
    card_index("bd002797-a545-4bee-88bf-b878436e7cca")
}
fn eden() -> CardIndex {
    card_index("84856b92-5ce8-47f3-9a1c-78d6a3e26aca")
}
fn underworld_breach() -> CardIndex {
    card_index("27e0948b-9916-473b-8d8c-a51bdfbc7457")
}
fn lightning_bolt() -> CardIndex {
    card_index("4457ed35-7c10-48c8-9776-456485fdf070")
}

/// Plays `card` from hand and stops with its enter trigger on the stack.
fn play_to_its_trigger(
    engine: &mut Engine<RegistryLookup>,
    seat: PlayerId,
    card: CardIndex,
) -> ObjectId {
    let land = in_hand(engine, seat, card).expect("the land is in hand");
    engine
        .apply(seat, PlayerAction::PlayLand { card: land })
        .expect("a land drop in the first main phase");
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Stack).len(),
        1,
        "the enter trigger is on the stack and nothing else is"
    );
    land
}

/// Every stack object whose ability is synthetic and whose source is `source`.
fn synthetic_on_stack(engine: &Engine<RegistryLookup>, source: ObjectId) -> Vec<ObjectId> {
    engine
        .state()
        .zones
        .list(ZoneLocation::Stack)
        .iter()
        .copied()
        .filter(|id| {
            engine
                .state()
                .object(*id)
                .and_then(|o| o.ability)
                .is_some_and(|loc| loc.index == AbilityRef::SYNTHETIC && loc.source == source)
        })
        .collect()
}

/// Passes priority until the stack is empty, and fails on any library search.
/// A test that expects no search cannot let a helper answer one on its
/// behalf.
fn resolve_without_a_search(engine: &mut Engine<RegistryLookup>) {
    for _ in 0..50 {
        if stack_is_empty(engine) {
            return;
        }
        assert!(
            engine.state().reflexive.is_empty(),
            "a reflexive trigger is never left in the state while a question is out"
        );
        match engine.pending().clone() {
            Pending::Priority { player, .. } => {
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            other => panic!("nothing should be asked here, and this was: {other:?}"),
        }
    }
    panic!("the stack never emptied");
}

/// Moves `object` into its owner's hand the way a bounce in response would.
fn bounce(engine: &mut Engine<RegistryLookup>, seat: PlayerId, object: ObjectId) {
    engine
        .dev_state_mut(seat)
        .expect("a test seat has dev commands")
        .move_object(
            object,
            ZoneLocation::Hand(seat),
            ZonePosition::Top,
            Cause::DevCommand,
        )
        .expect("the object moves");
}

/// CR 603.12: the reflexive ability fires only when its action happened,
/// and a land that has left the battlefield cannot be sacrificed.
///
/// Riveteers Overlook's enter trigger is on the stack when the land is
/// bounced. The trigger resolves, "sacrifice it" does nothing (CR 701.21a),
/// and so the reflexive ability is never created. Before, the search and the
/// life gain sat in the enter trigger's own effect list: the player searched
/// for a land and gained 1 life, and the land was moved from hand to
/// graveyard as if it had been sacrificed.
#[test]
fn a_reflexive_trigger_does_not_fire_when_its_action_was_impossible() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, swamp())
        .hand(0, &[riveteers_overlook()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let life = engine.state().players[0].life;
    let library = engine.state().zones.list(ZoneLocation::Library(p0)).len();

    let land = play_to_its_trigger(&mut engine, p0, riveteers_overlook());
    bounce(&mut engine, p0, land);
    resolve_without_a_search(&mut engine);

    assert_eq!(
        engine.state().object(land).map(|o| o.zone),
        Some(Zone::Hand),
        "the bounced land stays in hand: nothing sacrificed it from there"
    );
    assert_eq!(engine.state().players[0].life, life, "no life was gained");
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Library(p0)).len(),
        library,
        "and no land left the library"
    );
}

/// The same bounce against Brokers Hideout, the card that wrote its "when
/// you do" as a leaves-the-battlefield trigger.
///
/// That reading fired on the bounce. It is the reading CR 603.12's Manticore
/// example rules out: the reflexive ability "triggers only when you
/// sacrifice … due to the original triggered ability, and not if you
/// sacrifice a creature for any other reason". A bounce is not even a
/// sacrifice.
#[test]
fn brokers_hideout_bounced_in_response_searches_nothing() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[brokers_hideout()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let life = engine.state().players[0].life;

    let land = play_to_its_trigger(&mut engine, p0, brokers_hideout());
    bounce(&mut engine, p0, land);
    resolve_without_a_search(&mut engine);

    assert_eq!(
        engine.state().object(land).map(|o| o.zone),
        Some(Zone::Hand)
    );
    assert_eq!(engine.state().players[0].life, life);
}

/// CR 701.21a, the zone half: "sacrifice this enchantment" does nothing
/// once the enchantment has left the battlefield.
///
/// Underworld Breach has no reflexive ability, and that is why it is used
/// here. The guard belongs to `SacrificeSelf` itself, so it applies to every
/// card in the pool that prints the effect, not only to the reflexive ones.
/// Before, Breach was moved from hand to graveyard.
#[test]
fn sacrifice_self_refuses_a_source_that_has_left_the_battlefield() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[underworld_breach()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    pass_until(&mut engine, |e| {
        matches!(e.state().turn.phase, Phase::Ending) && !stack_is_empty(e)
    });
    let breach = on_battlefield(&engine, p0, underworld_breach()).expect("still in play");
    bounce(&mut engine, p0, breach);
    let version = engine.state().object(breach).map(|o| o.version);
    let before = engine.state().journal.last_seq();

    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().object(breach).map(|o| o.zone),
        Some(Zone::Hand)
    );
    assert_eq!(
        engine.state().object(breach).map(|o| o.version),
        version,
        "the card did not move again, so it is still the same object (CR 400.7)"
    );
    let moved_again = engine.state().journal.entries()[before as usize..]
        .iter()
        .any(|e| matches!(e.event, GameEvent::ZoneChanged { object, .. } if object == breach));
    assert!(
        !moved_again,
        "no zone change was journaled for it after the bounce"
    );
}

/// CR 701.21a, the controller half: "A player can't sacrifice … a permanent
/// they don't control."
///
/// Brokers Hideout changes control while its enter trigger is on the stack.
/// The trigger's controller is still the player who played the land, and the
/// land now belongs to the other seat's side of the table. So the sacrifice
/// does nothing, and neither does the reflexive ability that waits for it. A
/// guard that checked the zone alone would pass the zone test above and
/// sacrifice the land here.
#[test]
fn sacrifice_self_refuses_a_permanent_its_controller_does_not_control() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[brokers_hideout()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let life = engine.state().players[0].life;

    let land = play_to_its_trigger(&mut engine, p0, brokers_hideout());
    engine
        .dev_state_mut(p0)
        .expect("dev commands")
        .object_mut(land)
        .expect("the land")
        .set_controller(p1);
    let before = engine.state().journal.last_seq();
    resolve_without_a_search(&mut engine);

    let obj = engine.state().object(land).expect("the land");
    assert_eq!(obj.zone, Zone::Battlefield, "the land was not sacrificed");
    assert_eq!(obj.controller, p1);
    let moved = engine.state().journal.entries()[before as usize..]
        .iter()
        .any(|e| matches!(e.event, GameEvent::ZoneChanged { object, .. } if object == land));
    assert!(!moved);
    assert_eq!(
        engine.state().players[0].life,
        life,
        "and nothing was fetched"
    );
}

/// The reflexive ability is a stack object of its own (CR 603.12, which
/// makes it follow the rules for delayed triggers, and CR 603.3).
///
/// After Cabaretti Courtyard's enter trigger resolves, the land is in the
/// graveyard and exactly one synthetic ability from it is on the stack. The
/// other player receives priority while it waits there, before any search
/// is asked. Removing it from the stack, as a counter would, means no search
/// and no life. Before, the search was part of the enter trigger's own
/// resolution, so nobody could respond to it.
#[test]
fn a_reflexive_trigger_is_a_stack_object_of_its_own() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[cabaretti_courtyard()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let life = engine.state().players[0].life;

    let land = play_to_its_trigger(&mut engine, p0, cabaretti_courtyard());
    // Both players pass on the enter trigger, which resolves.
    let Pending::Priority { player, .. } = engine.pending().clone() else {
        panic!("priority with the enter trigger on the stack");
    };
    engine.apply(player, PlayerAction::PassPriority).unwrap();
    let Pending::Priority { player, .. } = engine.pending().clone() else {
        panic!("the other player's priority");
    };
    engine.apply(player, PlayerAction::PassPriority).unwrap();

    assert_eq!(
        engine.state().object(land).map(|o| o.zone),
        Some(Zone::Graveyard),
        "the enter trigger sacrificed the land"
    );
    let reflexive = synthetic_on_stack(&engine, land);
    assert_eq!(
        reflexive.len(),
        1,
        "one reflexive ability, from the land, waits on the stack"
    );
    assert_eq!(engine.state().zones.list(ZoneLocation::Stack).len(), 1);
    assert!(
        engine.state().reflexive.is_empty(),
        "it has left the state's holding list"
    );

    // p0 passes. p1 then holds priority with the ability still on the
    // stack, which is the window a player can respond in.
    let Pending::Priority { player, .. } = engine.pending().clone() else {
        panic!("priority with the reflexive ability on the stack");
    };
    assert_eq!(player, p0);
    engine.apply(p0, PlayerAction::PassPriority).unwrap();
    assert!(
        matches!(engine.pending(), Pending::Priority { player, .. } if *player == p1),
        "the opponent gets priority before anything is searched: {:?}",
        engine.pending()
    );

    // Taken off the stack, it does nothing.
    let state = engine.dev_state_mut(p0).expect("dev commands");
    state.zones.remove(reflexive[0], ZoneLocation::Stack);
    let _ = state.arena.remove(reflexive[0]);
    engine.apply(p1, PlayerAction::PassPriority).unwrap();
    resolve_without_a_search(&mut engine);
    assert_eq!(
        engine.state().players[0].life,
        life,
        "a countered reflexive gains nothing"
    );
}

/// A duel with Eden and five Forests in play for seat 0, whose library is
/// made of `filler`, and with `graveyard` put into seat 0's graveyard first.
fn eden_game(filler: CardIndex, graveyard: &[CardIndex]) -> (Engine<RegistryLookup>, ObjectId) {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, filler)
        .battlefield(
            0,
            &[eden(), forest(), forest(), forest(), forest(), forest()],
        )
        .hand(0, graveyard)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    for card in graveyard {
        let id = in_hand(&engine, p0, *card).expect("dealt to hand first");
        engine
            .dev_state_mut(p0)
            .expect("dev commands")
            .move_object(
                id,
                ZoneLocation::Graveyard(p0),
                ZonePosition::Top,
                Cause::DevCommand,
            )
            .expect("put into the graveyard");
    }
    let eden = on_battlefield(&engine, p0, eden()).expect("Eden is in play");
    (engine, eden)
}

/// Floats five mana off the Forests and activates Eden's `{5}, {T}`.
fn activate_eden(engine: &mut Engine<RegistryLookup>, p0: PlayerId, eden: ObjectId) {
    tap_mana_except(engine, p0, eden);
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: eden,
                ability_index: 1,
            },
        )
        .expect("the ability is paid out of the pool");
}

/// Passes until Eden's "you may sacrifice this land" is asked, and answers.
fn answer_the_may(engine: &mut Engine<RegistryLookup>, p0: PlayerId, yes: bool) {
    pass_until(engine, |e| {
        matches!(
            e.pending(),
            Pending::YesNo {
                prompt: crate::choice::YesNoPrompt::MayDo,
                ..
            }
        )
    });
    assert!(engine.state().reflexive.is_empty());
    engine.apply(p0, PlayerAction::YesNo(yes)).unwrap();
}

/// The reflexive target is chosen as the ability goes on the stack
/// (CR 603.3d, which sends it through CR 601.2c), and that happens after the
/// resolution that created it.
///
/// Eden mills two Forests and is sacrificed. The target question that
/// follows offers both Forests the mill just put into the graveyard. It
/// offers neither Eden, which "another" excludes, nor the Lightning Bolt,
/// which is not a permanent card (CR 110.4a). The chosen Forest is returned
/// to hand.
#[test]
fn a_reflexive_trigger_chooses_its_target_as_it_goes_on_the_stack() {
    let p0 = PlayerId::new(0);
    let (mut engine, eden) = eden_game(forest(), &[lightning_bolt()]);
    let bolt = in_graveyard(&engine, p0, lightning_bolt()).expect("the Bolt");
    let library: Vec<ObjectId> = engine.state().zones.list(ZoneLocation::Library(p0)).clone();
    let milled: Vec<ObjectId> = library.iter().rev().take(2).copied().collect();

    activate_eden(&mut engine, p0, eden);
    answer_the_may(&mut engine, p0, true);

    let Pending::ChooseTargets {
        options, player, ..
    } = engine.pending().clone()
    else {
        panic!(
            "the reflexive ability asks for its target: {:?}",
            engine.pending()
        );
    };
    assert_eq!(player, p0);
    assert!(engine.state().reflexive.is_empty());
    assert_eq!(
        engine.state().object(eden).map(|o| o.zone),
        Some(Zone::Graveyard)
    );
    for card in &milled {
        assert_eq!(
            engine.state().object(*card).map(|o| o.zone),
            Some(Zone::Graveyard)
        );
        assert!(
            options.contains(card),
            "a card the mill just put there is a legal target"
        );
    }
    assert!(!options.contains(&eden), "\"another\" leaves Eden out");
    assert!(
        !options.contains(&bolt),
        "an instant is not a permanent card"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![milled[0]],
                players: vec![],
            },
        )
        .unwrap();
    assert_eq!(synthetic_on_stack(&engine, eden).len(), 1);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().object(milled[0]).map(|o| o.zone),
        Some(Zone::Hand)
    );
    assert_eq!(
        engine.state().object(milled[1]).map(|o| o.zone),
        Some(Zone::Graveyard)
    );
}

/// CR 603.3d: a reflexive ability with no legal target is removed.
///
/// A library of Lightning Bolts mills two instants, so once Eden itself is
/// left out there is no permanent card to target. The sacrifice still
/// happens, no target is asked, and nothing waits on the stack.
#[test]
fn a_reflexive_trigger_with_no_legal_target_is_removed() {
    let p0 = PlayerId::new(0);
    let (mut engine, eden) = eden_game(lightning_bolt(), &[]);
    activate_eden(&mut engine, p0, eden);
    answer_the_may(&mut engine, p0, true);
    assert!(
        !matches!(engine.pending(), Pending::ChooseTargets { .. }),
        "nothing to point at, so nothing is asked"
    );
    assert!(stack_is_empty(&engine));
    assert_eq!(
        engine.state().object(eden).map(|o| o.zone),
        Some(Zone::Graveyard)
    );
}

/// Declining the "may" is not the action, so the reflexive ability is never
/// created.
#[test]
fn declining_the_may_creates_no_reflexive_trigger() {
    let p0 = PlayerId::new(0);
    let (mut engine, eden) = eden_game(forest(), &[]);
    activate_eden(&mut engine, p0, eden);
    answer_the_may(&mut engine, p0, false);
    assert!(!matches!(engine.pending(), Pending::ChooseTargets { .. }));
    assert!(stack_is_empty(&engine));
    let obj = engine.state().object(eden).expect("Eden");
    assert_eq!(obj.zone, Zone::Battlefield, "Eden stays");
    assert!(
        obj.status.contains(crate::object::Status::TAPPED),
        "tapped by its own cost"
    );
}

/// CR 608.2d: "The player can't choose an option that's illegal or
/// impossible." Once Eden has left the battlefield, sacrificing it is
/// impossible, so no "you may sacrifice this land" is asked. The two cards
/// are still milled.
#[test]
fn a_may_sacrifice_of_a_departed_source_asks_nothing() {
    let p0 = PlayerId::new(0);
    let (mut engine, eden) = eden_game(forest(), &[]);
    let library = engine.state().zones.list(ZoneLocation::Library(p0)).len();
    activate_eden(&mut engine, p0, eden);
    bounce(&mut engine, p0, eden);
    resolve_without_a_search(&mut engine);
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Library(p0)).len(),
        library - 2
    );
    assert_eq!(
        engine.state().object(eden).map(|o| o.zone),
        Some(Zone::Hand)
    );
}

/// CR 608.2b on a synthetic stack object: "A target that's no longer in the
/// zone it was in when it was targeted is illegal."
///
/// Eden's reflexive ability targets a milled Forest, and the Forest is
/// exiled before the ability resolves. Every target the ability had is now
/// illegal, so it does not resolve and the Forest stays in exile. Before,
/// `stack_target_req` answered `None` for every synthetic ability, the
/// re-check was never asked, and the Forest went from exile to hand.
#[test]
fn a_synthetic_trigger_rechecks_its_target_on_resolution() {
    let p0 = PlayerId::new(0);
    let (mut engine, eden) = eden_game(forest(), &[]);
    let library: Vec<ObjectId> = engine.state().zones.list(ZoneLocation::Library(p0)).clone();
    let top = *library.last().expect("a library");

    activate_eden(&mut engine, p0, eden);
    answer_the_may(&mut engine, p0, true);
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![top],
                players: vec![],
            },
        )
        .unwrap();
    let reflexive = synthetic_on_stack(&engine, eden);
    assert_eq!(reflexive.len(), 1);
    engine
        .dev_state_mut(p0)
        .expect("dev commands")
        .move_object(
            top,
            ZoneLocation::Exile(p0),
            ZonePosition::Top,
            Cause::DevCommand,
        )
        .expect("exiled in response");

    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().object(top).map(|o| o.zone),
        Some(Zone::Exile)
    );
    assert!(
        engine
            .state()
            .journal
            .entries()
            .iter()
            .any(|e| matches!(e.event, GameEvent::StackObjectDidNotResolve { object } if object == reflexive[0])),
        "the ability did not resolve"
    );
}
