//! `cards/creatures/mv_3/raven_familiar.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn raven_familiar() -> CardIndex {
    card_index("47aabd9d-0f68-490a-a2ae-5cd9ef689be6")
}

#[test]
fn raven_familiar_etb_picks_one_of_top_three() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(), island(), island()])
        .hand(0, &[raven_familiar()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();
    cast_from_hand(&mut engine, p0, raven_familiar());

    let Pending::ChooseCards { options, .. } = pass_to_card_choice(&mut engine) else {
        panic!("expected ChooseCards for Raven Familiar");
    };
    assert_eq!(options.len(), 3);
    let expected_top: Vec<_> = engine
        .state()
        .zones
        .list(ZoneLocation::Library(p0))
        .iter()
        .rev()
        .take(3)
        .copied()
        .collect();
    assert_eq!(options, expected_top);
    let chosen = options[0];
    let mut remainder = options[1..].to_vec();
    remainder.reverse();
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![chosen],
            },
        )
        .unwrap();

    let Pending::Arrange { cards, .. } = engine.pending().clone() else {
        panic!("order the other two cards")
    };
    assert_eq!(cards.len(), 2);
    engine
        .apply(
            p0,
            PlayerAction::Arrange {
                piles: vec![remainder.clone()],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    let hand_after = engine.state().zones.list(ZoneLocation::Hand(p0));
    assert_eq!(hand_after.len(), hand_before);
    assert!(hand_after.contains(&chosen));
    let library = engine.state().zones.list(ZoneLocation::Library(p0));
    assert!(
        library[..2].iter().all(|id| remainder.contains(id)),
        "the two unpicked cards are on the bottom"
    );
    // Its echo upkeep is a real price, even after its entry benefit resolved.
    pass_until(&mut engine, |e| {
        e.state().turn.active == p0
            && e.state().turn.step == crate::turn::Step::Upkeep
            && matches!(e.pending(), Pending::Priority { .. })
    });
    tap_all_mana(&mut engine, p0);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    engine.apply(p0, PlayerAction::YesNo(false)).unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert!(in_graveyard(&engine, p0, raven_familiar()).is_some());
}

#[test]
fn flying_prevents_a_ground_creature_from_blocking() {
    use crate::engine::card_tests::legends_kit as lk;
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[raven_familiar(), island(), island(), island()])
        .battlefield(1, &[gray_ogre()])
        .start();
    keep_mulligans(&mut engine);
    pass_until(&mut engine, |e| {
        e.state().turn.active == p0
            && e.state().turn.step == crate::turn::Step::Upkeep
            && matches!(e.pending(), Pending::Priority { .. })
    });
    tap_all_mana(&mut engine, p0);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    engine.apply(p0, PlayerAction::YesNo(true)).unwrap();
    reach_main_phase(&mut engine, p0);
    let flyer = lk::obj(&engine, lk::P0, raven_familiar());
    let ground = lk::obj(&engine, lk::P1, gray_ogre());
    let offer = lk::attack(&mut engine, &[flyer]);
    assert!(!lk::may_block(&offer, ground, flyer));
}
