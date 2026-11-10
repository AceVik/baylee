#[allow(clippy::wildcard_imports)]
use super::*;

fn brand() -> CardIndex {
    card_index("eec7500b-418d-4387-beb2-d53e047c1d22")
}

#[test]
fn brand_regains_control_of_permanents_you_own() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, mountain())
        .hand(0, &[brand()])
        .battlefield(0, &[mountain(), grizzly_bears()])
        .battlefield(1, &[gray_ogre()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let bears = on_battlefield(&engine, p0, grizzly_bears()).expect("seated");
    engine.state.object_mut(bears).unwrap().controller = p1;
    engine.state.invalidate_projections();

    cast_from_hand(&mut engine, p0, brand());
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(engine.state().object(bears).unwrap().controller, p0);
    assert!(on_battlefield(&engine, p1, gray_ogre()).is_some());
}

#[test]
fn brand_cycles_from_hand_for_two_mana() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[brand()])
        .battlefield(0, &[forest(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();
    tap_all_mana(&mut engine, p0);
    let card = in_hand(&engine, p0, brand()).unwrap();
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: card,
                ability_index: 1,
            },
        )
        .unwrap();
    assert!(in_graveyard(&engine, p0, brand()).is_some());
    assert_eq!(engine.state().players[0].mana_pool.total(), 0);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        before
    );
}
