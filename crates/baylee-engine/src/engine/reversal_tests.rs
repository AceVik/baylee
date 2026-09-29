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
