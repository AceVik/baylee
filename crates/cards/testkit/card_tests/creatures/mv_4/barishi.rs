//! `cards/creatures/mv_4/barishi.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::GRIZZLY_BEARS;

fn barishi() -> CardIndex {
    card_index("ffa93a79-7a10-4331-91de-eabbc4fee17e")
}

/// `Barishi` ({2}{G}{G}, 4/3 Creature — Elemental, `Coverage::Partial`) exiles
/// itself when it dies.
#[test]
fn barishi_exiles_itself_when_it_dies() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[mountain(), barishi()])
        .hand(0, &[lightning_bolt()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let id = on_battlefield(&engine, p0, barishi()).unwrap();

    cast_from_hand(&mut engine, p0, lightning_bolt());
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![id],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(in_graveyard(&engine, p0, barishi()).is_none());
    assert_eq!(engine.state().zones.list(ZoneLocation::Exile(p0)).len(), 1);
}

/// "When this creature dies" is Barishi's own death: another creature dying
/// beside it goes to the graveyard and Barishi stays on the battlefield.
#[test]
fn barishi_does_not_exile_when_another_creature_dies() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[mountain(), barishi(), GRIZZLY_BEARS])
        .hand(0, &[lightning_bolt()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let bears = on_battlefield(&engine, p0, GRIZZLY_BEARS).unwrap();

    cast_from_hand(&mut engine, p0, lightning_bolt());
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![bears],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(in_graveyard(&engine, p0, GRIZZLY_BEARS).is_some());
    assert!(on_battlefield(&engine, p0, barishi()).is_some());
    assert!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Exile(p0))
            .is_empty()
    );
}
