#[allow(clippy::wildcard_imports)]
use super::*;

fn wayward_soul() -> CardIndex {
    card_index("319b4715-c3eb-49bf-b8d2-4d41e601ebe4")
}

#[test]
fn wayward_soul_puts_itself_on_top_of_owners_library() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, island())
        .hand(0, &[wayward_soul()])
        .battlefield(0, &[island(), island(), island(), island(), island()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, wayward_soul());
    pass_until(&mut engine, stack_is_empty);
    let soul = on_battlefield(&engine, p0, wayward_soul()).expect("seated");
    let lib_before = engine.state().zones.list(ZoneLocation::Library(p0)).len();

    tap_all_mana(&mut engine, p0);
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: soul,
                ability_index: 0,
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p0, wayward_soul()).is_none());
    let lib_after = engine.state().zones.list(ZoneLocation::Library(p0));
    assert_eq!(lib_after.len(), lib_before + 1);
    let top = *lib_after.last().unwrap();
    assert_eq!(
        engine.state().object(top).unwrap().card.unwrap().index,
        wayward_soul()
    );
}

#[test]
fn wayward_soul_flies_over_ground_creatures() {
    use crate::engine::card_tests::legends_kit as lk;
    use baylee_core::generated::index;
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = lk::game(
        SEED,
        &[wayward_soul()],
        &[gray_ogre(), index::WIND_DRAKE],
        &[],
        &[],
    );
    let soul = lk::obj(&e, p0, wayward_soul());
    let ground = lk::obj(&e, p1, gray_ogre());
    let flyer = lk::obj(&e, p1, index::WIND_DRAKE);
    let offer = lk::attack(&mut e, &[soul]);
    assert!(!lk::may_block(&offer, ground, soul));
    assert!(lk::may_block(&offer, flyer, soul));
}
