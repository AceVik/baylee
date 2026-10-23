#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;

fn veiled_apparition() -> CardIndex {
    card_index("4187fd6d-5660-4ea7-a9dd-402487ec8721")
}

#[test]
fn veiled_apparition_becomes_illusion_creature_when_opponent_casts_spell() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[veiled_apparition()])
        .hand(1, &[gray_ogre()])
        .battlefield(1, &[mountain(), mountain(), mountain()])
        .start();
    keep_mulligans(&mut engine);
    reach_their_main_phase(&mut engine, p1);

    cast_from_hand(&mut engine, p1, gray_ogre());
    pass_until(&mut engine, stack_is_empty);

    let illusion = on_battlefield(&engine, p0, veiled_apparition()).expect("apparition on bf");
    assert_eq!(pt(&engine, illusion), (3, 3));
    assert!(keywords(&engine, illusion).contains(KeywordSet::FLYING));
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    engine.apply(p0, PlayerAction::YesNo(false)).unwrap();
    reach_main_phase(&mut engine, p0);
    assert!(in_graveyard(&engine, p0, veiled_apparition()).is_some());
}

#[test]
fn veiled_apparition_can_pay_its_colored_upkeep_cost_after_animation() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = lk::game(
        SEED,
        &[veiled_apparition(), island(), island()],
        &[],
        &[],
        &[gray_ogre()],
    );
    reach_their_main_phase(&mut engine, p1);
    lk::float(&mut engine, p1, &[(ManaColor::Red, 3)]);
    lk::cast_at(&mut engine, p1, gray_ogre(), &[], &[]);
    pass_until(&mut engine, |e| {
        e.state().turn.active == p0
            && e.state().turn.step == crate::turn::Step::Upkeep
            && matches!(e.pending(), Pending::Priority { .. })
    });
    tap_all_mana(&mut engine, p0);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    assert!(matches!(engine.pending(), Pending::YesNo { player, .. } if *player == p0));
    lk::yes_no(&mut engine, true);
    reach_main_phase(&mut engine, p0);
    assert!(on_battlefield(&engine, p0, veiled_apparition()).is_some());
    assert_eq!(lk::pool_total(&engine, p0), 0);
}
