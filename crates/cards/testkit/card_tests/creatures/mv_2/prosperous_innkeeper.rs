//! `cards/creatures/mv_2/prosperous_innkeeper.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn prosperous_innkeeper() -> CardIndex {
    card_index("da785227-cf8a-4d44-9e7c-fc909ea868f2")
}

fn treasures(engine: &Engine<RegistryLookup>, seat: PlayerId) -> Vec<ObjectId> {
    tokens_of(engine, seat)
        .into_iter()
        .filter(|id| {
            engine
                .state()
                .object(*id)
                .and_then(|object| object.token)
                .is_some_and(|token| token.name == "Treasure")
        })
        .collect()
}

/// "When this creature enters, create a Treasure token." The creature is cast
/// for its printed cost, and its own arrival does not also satisfy "another
/// creature you control enters" for the life trigger.
#[test]
fn prosperous_innkeeper_creates_one_treasure_but_gains_no_life_for_itself() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest()])
        .hand(0, &[prosperous_innkeeper()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, prosperous_innkeeper());
    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, p0, prosperous_innkeeper()).is_some(),
        "the Innkeeper resolved onto the battlefield"
    );
    let treasures = treasures(&engine, p0);
    assert_eq!(treasures.len(), 1, "its entry creates exactly one Treasure");
    let treasure = engine.state().object(treasures[0]).unwrap();
    let characteristics = treasure.characteristics();
    assert_eq!(engine.state().names.get(characteristics.name), "Treasure");
    assert!(characteristics.types.contains(TypeSet::ARTIFACT));
    assert!(!characteristics.types.contains(TypeSet::CREATURE));
    assert_eq!(
        engine.state().players[0].life,
        20,
        "it is not another creature to itself"
    );
    assert_eq!(
        engine.state().players[1].life,
        20,
        "the opponent gains nothing"
    );
}

/// Each later creature its controller brings in gains exactly one life. An
/// artifact does not trigger, and a second Innkeeper gives its controller one
/// life (from the copy already present), not two from its own ability as well.
#[test]
fn prosperous_innkeeper_gains_one_life_for_each_other_creature_only() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                prosperous_innkeeper(),
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
            ],
        )
        .hand(0, &[llanowar_elves(), sol_ring(), prosperous_innkeeper()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, llanowar_elves());
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().players[0].life,
        21,
        "the Elves trigger the Innkeeper"
    );

    cast_from_hand(&mut engine, p0, sol_ring());
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().players[0].life,
        21,
        "an artifact is not a creature"
    );

    cast_from_hand(&mut engine, p0, prosperous_innkeeper());
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        all_on_battlefield(&engine, p0, prosperous_innkeeper()).len(),
        2
    );
    assert_eq!(
        engine.state().players[0].life,
        22,
        "only the Innkeeper already on the battlefield triggers"
    );
    assert_eq!(
        treasures(&engine, p0).len(),
        1,
        "the second Innkeeper creates its own Treasure"
    );
}

/// A creature entering under the opponent's control is not "another creature
/// you control", even though the Innkeeper's controller owns no creature that
/// entered instead: there simply is no trigger and no life gain.
#[test]
fn prosperous_innkeeper_ignores_creatures_controlled_by_an_opponent() {
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[prosperous_innkeeper()])
        .battlefield(1, &[forest(), forest()])
        .hand(1, &[grizzly_bears()])
        .start();
    keep_mulligans(&mut engine);
    reach_their_main_phase(&mut engine, p1);

    cast_from_hand(&mut engine, p1, grizzly_bears());
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p1, grizzly_bears()).is_some());
    assert_eq!(engine.state().players[0].life, 20);
    assert_eq!(engine.state().players[1].life, 20);
}
