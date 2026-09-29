//! A reversed action gives back what was paid for it (CR 732.1).
//!
//! "If a player takes an illegal action or starts to take an action but
//! can't legally complete it, the entire action is reversed and any payments
//! already made are canceled." A cast and an activation are paid part by
//! part, the mana first and all at once, and a part after it could refuse:
//! the cast was then reversed and the activation handed priority back with
//! the mana spent and the costs paid by then left paid.
//!
//! No card in the pool reaches that today by itself. Every question is asked
//! before anything is paid, and every answer is checked against the list the
//! question offered, so the parts that can refuse after the mana (a move of
//! an object that is gone, a counter that is not there) refuse only for an
//! object that stopped existing between the question and its answer. The
//! tests here make it stop existing, with the harness' own capability, to
//! reach the parts the payment has and the pool does not yet use.

#[allow(clippy::wildcard_imports)] // the shared duel plumbing, as every card test takes it
use super::testkit::*;

#[allow(clippy::wildcard_imports)] // this module's own vocabulary
use super::*;

use crate::zone::ZoneLocation;
use baylee_core::ids::{CardIndex, ObjectId};

fn forest() -> CardIndex {
    card_index("b34bb2dc-c1af-4d77-b0b3-a0fb342a5fc6")
}

fn llanowar_elves() -> CardIndex {
    card_index("68954295-54e3-4303-a6bc-fc4547a4e3a3")
}

/// "{2}, {T}, Sacrifice a creature: Draw a card."
fn phyrexian_vault() -> CardIndex {
    card_index("b628150b-08a1-4ea3-978d-60255dfb0b7e")
}

/// "As an additional cost to cast this spell, sacrifice a green creature."
fn natural_order() -> CardIndex {
    card_index("8c1fe337-375a-4add-93b6-0ac39ed72b4f")
}

fn plains() -> CardIndex {
    card_index("bc71ebf6-2056-41f7-be35-b2e5c34afa99")
}

/// "When this creature enters, exile up to one other target non-Fox
/// creature until this creature leaves the battlefield."
fn werefox_bodyguard() -> CardIndex {
    card_index("d5ee2ced-29f4-430f-962e-2f930b92624c")
}

/// Takes `object` off the battlefield and out of the game altogether, as a
/// token that left it does (CR 111.7): whatever still names it names nothing.
#[track_caller]
fn cease(engine: &mut Engine<RegistryLookup>, seat: PlayerId, object: ObjectId) {
    let state = engine
        .dev_state_mut(seat)
        .expect("the harness may set boards up");
    assert!(state.zones.remove(object, ZoneLocation::Battlefield));
    assert!(state.arena.remove(object).is_some());
}

fn pool(engine: &Engine<RegistryLookup>, seat: PlayerId) -> u32 {
    engine.state().players[seat.get() as usize]
        .mana_pool
        .total()
}

/// Phyrexian Vault answered with a creature that is gone: the activation is
/// reversed with its {2} back in the pool and the Vault untapped.
#[test]
fn a_reversed_activation_gives_its_mana_and_its_tap_back() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(3, forest())
        .battlefield(
            0,
            &[
                phyrexian_vault(),
                forest(),
                forest(),
                llanowar_elves(),
                llanowar_elves(),
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let vault = on_battlefield(&engine, p0, phyrexian_vault()).expect("the Vault");
    let forests: Vec<ObjectId> = engine
        .state()
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .copied()
        .filter(|id| {
            engine
                .state()
                .object(*id)
                .is_some_and(|o| o.card.is_some_and(|c| c.index == forest()))
        })
        .collect();
    tap_mana_where(&mut engine, p0, |id| forests.contains(&id));
    assert_eq!(pool(&engine, p0), 2);
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(vault, 0)),
        "{:?}",
        legal.abilities
    );
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: vault,
                ability_index: 0,
            },
        )
        .expect("the Vault activates");
    let Pending::ChooseCards { options, .. } = engine.pending().clone() else {
        panic!("the cost asks which creature, got {:?}", engine.pending())
    };
    assert_eq!(options.len(), 2, "the two Elves: {options:?}");
    let gone = options[0];
    cease(&mut engine, p0, gone);

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![gone],
            },
        )
        .expect("an answer the question offered is taken");

    assert!(
        matches!(engine.pending(), Pending::Priority { player, .. } if *player == p0),
        "the activation is reversed and priority is the activator's: {:?}",
        engine.pending()
    );
    assert!(stack_is_empty(&engine), "nothing was activated");
    assert_eq!(pool(&engine, p0), 2, "the {{2}} is back in the pool");
    assert!(
        !engine
            .state()
            .object(vault)
            .expect("the Vault")
            .status
            .contains(crate::object::Status::TAPPED),
        "the {{T}} is paid back"
    );
    assert!(
        on_battlefield(&engine, p0, llanowar_elves()).is_some(),
        "the other Elves stand"
    );
}

/// Natural Order answered with a green creature that is gone: the cast is
/// reversed with its {2}{G}{G} back in the pool and the card back in hand.
#[test]
fn a_reversed_cast_gives_its_mana_back() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(3, forest())
        .battlefield(
            0,
            &[
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
                llanowar_elves(),
            ],
        )
        .hand(0, &[natural_order()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let elves = on_battlefield(&engine, p0, llanowar_elves()).expect("the Elves");
    let forests: Vec<ObjectId> = engine
        .state()
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .copied()
        .filter(|id| *id != elves)
        .collect();
    tap_mana_where(&mut engine, p0, |id| forests.contains(&id));
    assert_eq!(pool(&engine, p0), 5);
    cast_with_floating(&mut engine, p0, natural_order());
    let Pending::ChooseCards { options, .. } = engine.pending().clone() else {
        panic!("the cost asks which creature, got {:?}", engine.pending())
    };
    assert_eq!(options, vec![elves]);
    cease(&mut engine, p0, elves);

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![elves],
            },
        )
        .expect("an answer the question offered is taken");

    assert!(
        matches!(engine.pending(), Pending::Priority { player, .. } if *player == p0),
        "the cast is reversed and priority is the caster's: {:?}",
        engine.pending()
    );
    assert!(stack_is_empty(&engine), "nothing was cast");
    assert!(
        in_hand(&engine, p0, natural_order()).is_some(),
        "the spell is back where it came from"
    );
    assert_eq!(
        pool(&engine, p0),
        5,
        "the {{2}}{{G}}{{G}} is back in the pool"
    );
}

/// A reversed sacrifice puts back what its leaving set free.
///
/// Werefox Bodyguard holds seat 1's Elves in exile until it leaves the
/// battlefield, and they return inside the move that takes it away (CR
/// 610.3, `GameState::move_object`), so a cost that sacrifices it sets them
/// free while the payment is still running. When a later part of that cost
/// refuses, the whole action is reversed (CR 732.1): the Bodyguard stands,
/// the Elves are in exile again as the object they were and held by it, and
/// the journal has neither move. No card prints a sacrifice followed by a
/// part that can refuse; this cost is written for the test, and asks for a
/// counter the Bodyguard does not have.
#[test]
fn a_reversed_sacrifice_puts_back_what_its_leaving_set_free() {
    static SACRIFICE_THEN_A_COUNTER: [CostPart; 2] = [
        CostPart::SacrificeSelf,
        CostPart::RemoveCounterSelf {
            kind: baylee_cards_dsl::CounterKind::P1P1,
            n: 1,
        },
    ];
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(614, plains())
        .battlefield(0, &[plains(), plains(), plains()])
        .battlefield(1, &[llanowar_elves()])
        .hand(0, &[werefox_bodyguard()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let elves = on_battlefield(&engine, p1, llanowar_elves()).expect("seat 1's Elves");
    cast_from_hand(&mut engine, p0, werefox_bodyguard());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![elves],
                players: vec![],
            },
        )
        .expect("the Elves are a target");
    pass_until(&mut engine, stack_is_empty);
    let bodyguard = on_battlefield(&engine, p0, werefox_bodyguard()).expect("the Bodyguard");
    let held = |engine: &Engine<RegistryLookup>| {
        engine.state().object(elves).is_some_and(|o| {
            o.zone == crate::zone::Zone::Exile
                && o.riders.iter().any(
                    |r| matches!(r, crate::object::Rider::Linked { host, .. } if *host == bodyguard),
                )
        })
    };
    assert!(held(&engine), "the Bodyguard holds the Elves");
    let version = engine.state().object(elves).map(|o| o.version);
    let journal = engine.state().journal.len();

    let cost = Cost {
        mana: baylee_core::mana::ManaCost::ZERO,
        parts: &SACRIFICE_THEN_A_COUNTER,
    };
    assert!(
        engine.pay_cost(p0, bodyguard, &cost, &[], 0).is_err(),
        "the Bodyguard has no counter to remove"
    );

    assert_eq!(
        engine.state().object(bodyguard).map(|o| o.zone),
        Some(crate::zone::Zone::Battlefield),
        "the sacrifice is reversed"
    );
    assert!(held(&engine), "and the Bodyguard holds the Elves again");
    assert_eq!(
        engine.state().object(elves).map(|o| o.version),
        version,
        "the object they were"
    );
    assert_eq!(
        engine.state().journal.len(),
        journal,
        "neither move is in the journal"
    );
}

/// "Escape—{G}{G}{U}{U}, Exile five other cards from your graveyard."
fn uro_titan_of_nature_s_wrath() -> CardIndex {
    card_index("ee302659-59ed-4eef-babe-451b9ccf7f14")
}

fn island() -> CardIndex {
    card_index("b2c6aa39-2d2a-459c-a555-fb48ba993373")
}

/// Uro escaped with five other cards, the fifth of which is gone by the
/// time the answer is paid: four were exiled and the mana was spent before
/// the fifth refused. The cast is reversed (CR 732.1), and the four are back
/// in the graveyard as they were, Uro with them, and the {G}{G}{U}{U} back
/// in the pool. The gone card is named last on purpose, so the refusal
/// comes after exiles that a reversal without the roll-back would keep.
#[test]
fn a_reversed_escape_puts_the_exiled_cards_back() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(3, forest())
        .battlefield(0, &[forest(), forest(), island(), island()])
        .hand(0, &[uro_titan_of_nature_s_wrath()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let uro = in_hand(&engine, p0, uro_titan_of_nature_s_wrath()).expect("Uro in hand");
    {
        let state = engine
            .dev_state_mut(p0)
            .expect("the harness may set boards up");
        state
            .move_object(
                uro,
                ZoneLocation::Graveyard(p0),
                crate::zone::ZonePosition::Top,
                crate::event::Cause::Effect,
            )
            .expect("the harness moves a card");
    }
    seed_graveyard(&mut engine, p0, 5);
    tap_all_mana(&mut engine, p0);
    assert_eq!(pool(&engine, p0), 4);
    engine
        .apply(p0, PlayerAction::CastSpell { card: uro })
        .expect("Uro escapes");
    if let Pending::ChooseCastMode { options, .. } = engine.pending().clone() {
        let escape = options
            .iter()
            .position(|o| o.kind == crate::choice::CastModeKind::Escape)
            .expect("escape is offered");
        engine
            .apply(p0, PlayerAction::ChooseMode(escape))
            .expect("escape is chosen");
    }
    let Pending::ChooseCards {
        options,
        prompt: crate::choice::ChoicePrompt::CostExile,
        ..
    } = engine.pending().clone()
    else {
        panic!("escape asks which five, got {:?}", engine.pending())
    };
    assert_eq!(options.len(), 5, "the five other cards: {options:?}");
    let gone = options[4];
    {
        let state = engine
            .dev_state_mut(p0)
            .expect("the harness may set boards up");
        assert!(state.zones.remove(gone, ZoneLocation::Graveyard(p0)));
        assert!(state.arena.remove(gone).is_some());
    }

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: options.clone(),
            },
        )
        .expect("an answer the question offered is taken");

    assert!(
        matches!(engine.pending(), Pending::Priority { player, .. } if *player == p0),
        "the cast is reversed and priority is the caster's: {:?}",
        engine.pending()
    );
    assert!(stack_is_empty(&engine), "nothing was cast");
    let graveyard = engine.state().zones.list(ZoneLocation::Graveyard(p0));
    assert!(graveyard.contains(&uro), "Uro is back where it came from");
    for card in &options[..4] {
        assert!(
            graveyard.contains(card),
            "{card:?} was exiled for the cost and is back"
        );
    }
    assert!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Exile(p0))
            .is_empty(),
        "nothing stays exiled"
    );
    assert_eq!(
        pool(&engine, p0),
        4,
        "the {{G}}{{G}}{{U}}{{U}} is back in the pool"
    );
}
