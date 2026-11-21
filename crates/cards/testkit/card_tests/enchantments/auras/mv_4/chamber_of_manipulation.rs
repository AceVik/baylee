//! `cards/enchantments/auras/mv_4/chamber_of_manipulation.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;

fn chamber_of_manipulation() -> CardIndex {
    card_index("010e0d5c-e751-4f67-b305-8a598cc1f0de")
}

/// Enchanted land gains "{T}, Discard a card: Gain control of target creature
/// until end of turn."
#[test]
fn chamber_of_manipulation_steals_target_creature_until_end_of_turn() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = lk::game(
        SEED,
        &[island()],
        &[grizzly_bears()],
        &[chamber_of_manipulation(), forest()],
        &[],
    );
    let my_isl = lk::obj(&e, p0, island());
    let opp_bear = lk::obj(&e, p1, grizzly_bears());

    lk::float(
        &mut e,
        p0,
        &[(ManaColor::Blue, 2), (ManaColor::Colorless, 2)],
    );
    lk::cast_at(&mut e, p0, chamber_of_manipulation(), &[my_isl], &[]);

    let discard = in_hand(&e, p0, forest()).unwrap();
    activate(&mut e, p0, island(), baylee_engine::choice::GRANTED_ABILITY);
    lk::aim(&mut e, &[opp_bear], &[]);
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    e.apply(
        p0,
        PlayerAction::ChooseObjects {
            objects: vec![discard],
        },
    )
    .unwrap();
    lk::settle(&mut e);

    assert_eq!(e.state().object(opp_bear).unwrap().controller, p0);
    assert_eq!(e.state().object(my_isl).unwrap().controller, p0);

    reach_their_main_phase(&mut e, p1);
    assert_eq!(e.state().object(opp_bear).unwrap().controller, p1);
}
