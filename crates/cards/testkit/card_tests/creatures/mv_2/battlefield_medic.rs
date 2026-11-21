//! `cards/creatures/mv_2/battlefield_medic.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;

fn battlefield_medic() -> CardIndex {
    card_index("a600af0b-ea04-4813-b700-f51820868c86")
}

/// Battlefield Medic ({1}{W} 1/1 Human Cleric):
/// "{T}: Prevent the next X damage that would be dealt to target creature this
/// turn, where X is the number of Clerics on the battlefield."
#[test]
fn battlefield_medic_prevents_next_x_damage_counting_all_clerics_on_battlefield() {
    let mut e = lk::game(
        SEED,
        &[battlefield_medic(), grizzly_bears()],
        &[ondu_cleric()],
        &[lightning_bolt(), lightning_bolt()],
        &[],
    );
    let bear = lk::obj(&e, lk::P0, grizzly_bears());

    // 2 Clerics total on battlefield (yours + opponent's) -> prevents next 2 damage.
    lk::use_ability(&mut e, lk::P0, battlefield_medic(), 0, &[bear], &[]);

    lk::float(&mut e, lk::P0, &[(ManaColor::Red, 1)]);
    lk::cast_at(&mut e, lk::P0, lightning_bolt(), &[bear], &[]);
    assert!(
        on_battlefield(&e, lk::P0, grizzly_bears()).is_some(),
        "2 of Lightning Bolt's 3 damage were prevented, leaving 1 on the 2/2 Bear"
    );
    assert_eq!(lk::damage_on(&e, bear), 1);

    // Shield is exhausted; a second Bolt deals full damage and destroys the Bear.
    lk::float(&mut e, lk::P0, &[(ManaColor::Red, 1)]);
    lk::cast_at(&mut e, lk::P0, lightning_bolt(), &[bear], &[]);
    assert!(on_battlefield(&e, lk::P0, grizzly_bears()).is_none());
}
