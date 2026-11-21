//! `cards/creatures/mv_4/undying_beast.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn undying_beast() -> CardIndex {
    card_index("5ae03181-a436-4bad-be3a-6c9f6c0ed4d6")
}

/// Dying puts Undying Beast on top of its owner's library so the next draw
/// draws it back into hand.
#[test]
fn undying_beast_returns_to_top_of_owners_library_when_it_dies() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[swamp(), swamp(), swamp(), swamp(), mountain()])
        .hand(0, &[undying_beast(), lightning_bolt()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0));

    let swamps = all_on_battlefield(&engine, p0, swamp());
    tap_mana_where(&mut engine, p0, |id| swamps.contains(&id));
    let spell = in_hand(&engine, p0, undying_beast()).unwrap();
    engine
        .apply(p0, PlayerAction::CastSpell { card: spell })
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    let beast = on_battlefield(&engine, p0, undying_beast()).expect("resolved");
    assert_eq!(pt(&engine, beast), (3, 2));
    let lib_before = engine.state().zones.list(ZoneLocation::Library(p0)).len();

    let mtn = on_battlefield(&engine, p0, mountain()).unwrap();
    tap_mana_where(&mut engine, p0, |id| id == mtn);
    let bolt = in_hand(&engine, p0, lightning_bolt()).unwrap();
    engine
        .apply(p0, PlayerAction::CastSpell { card: bolt })
        .unwrap();
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![beast],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p0, undying_beast()).is_none());
    assert!(in_graveyard(&engine, p0, undying_beast()).is_none());
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Library(p0)).len(),
        lib_before + 1,
        "Undying Beast moved onto the top of its owner's library"
    );

    // Next p0 draw step draws Undying Beast off the top.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert!(in_hand(&engine, p0, undying_beast()).is_some());
}

/// Exiling Undying Beast from the battlefield is not dying (CR 700.4).
#[test]
fn undying_beast_does_not_return_to_library_when_exiled() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[undying_beast()])
        .battlefield(1, &[plains()])
        .hand(1, &[swords_to_plowshares()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let beast = on_battlefield(&engine, p0, undying_beast()).unwrap();
    let lib_before = engine.state().zones.list(ZoneLocation::Library(p0)).len();

    engine.apply(p0, PlayerAction::PassPriority).unwrap();
    cast_from_hand(&mut engine, p1, swords_to_plowshares());
    engine
        .apply(
            p1,
            PlayerAction::ChooseTargets {
                objects: vec![beast],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().zones.list(ZoneLocation::Library(p0)).len(),
        lib_before
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Exile(p0)).len(),
        1,
        "remains in exile"
    );
}
