//! `cards/creatures/mv_2/walking_sponge.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn walking_sponge() -> CardIndex {
    card_index("000e5d65-96c3-498b-bd01-72b1a1991850")
}

#[test]
fn walking_sponge_removes_flying_from_target_creature_until_end_of_turn() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let flyer = baylee_core::generated::index::STORM_CROW;
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[walking_sponge()])
        .battlefield(1, &[flyer])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let sponge = on_battlefield(&engine, p0, walking_sponge()).unwrap();
    let target = on_battlefield(&engine, p1, flyer).unwrap();
    assert!(keywords(&engine, target).contains(KeywordSet::FLYING));

    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: sponge,
                ability_index: 0,
            },
        )
        .unwrap();
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![target],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCastMode { .. })
    });
    engine.apply(p0, PlayerAction::ChooseMode(0)).unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(!keywords(&engine, target).contains(KeywordSet::FLYING));
    reach_their_main_phase(&mut engine, p1);
    assert!(keywords(&engine, target).contains(KeywordSet::FLYING));
}

#[test]
fn walking_sponge_can_choose_first_strike_or_trample_on_resolution() {
    use crate::engine::card_tests::legends_kit as lk;
    use baylee_core::generated::index;
    for (card, mode, keyword) in [
        (index::WHITE_KNIGHT, 1, KeywordSet::FIRST_STRIKE),
        (index::WAR_MAMMOTH, 2, KeywordSet::TRAMPLE),
    ] {
        let mut engine = lk::game(SEED, &[walking_sponge()], &[card], &[], &[]);
        let target = lk::obj(&engine, lk::P1, card);
        assert!(keywords(&engine, target).contains(keyword));
        activate(&mut engine, lk::P0, walking_sponge(), 0);
        lk::aim(&mut engine, &[target], &[]);
        pass_until(&mut engine, |e| {
            matches!(e.pending(), Pending::ChooseCastMode { .. })
        });
        engine
            .apply(lk::P0, PlayerAction::ChooseMode(mode))
            .unwrap();
        pass_until(&mut engine, stack_is_empty);
        assert!(!keywords(&engine, target).contains(keyword));
        reach_their_main_phase(&mut engine, lk::P1);
        assert!(keywords(&engine, target).contains(keyword));
    }
}
