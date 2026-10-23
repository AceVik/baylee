//! `cards/sorceries/mv_2/verdant_touch.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use crate::engine::card_tests::partials_w2c_kit::power_toughness;

const P0: PlayerId = PlayerId::new(0);

fn verdant_touch() -> CardIndex {
    card_index("68845200-5346-473d-becd-27315879443d")
}

/// Verdant Touch (`68845200-5346-473d-becd-27315879443d`) — {1}{G} Sorcery (Partial):
/// "Target land becomes a 2/2 creature that's still a land."
#[test]
fn verdant_touch_makes_target_land_a_two_two_creature() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest()])
        .hand(0, &[verdant_touch()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let land = on_battlefield(&engine, P0, forest()).unwrap();
    cast_from_hand(&mut engine, P0, verdant_touch());
    aim_at(&mut engine, P0, land);
    pass_until(&mut engine, stack_is_empty);
    assert!(
        in_graveyard(&engine, P0, verdant_touch()).is_some(),
        "base spell does not get free buyback"
    );

    assert!(types(&engine, land).contains(TypeSet::CREATURE));
    assert!(types(&engine, land).contains(TypeSet::LAND));
    assert_eq!(power_toughness(&engine, land), (2, 2));
    let other = all_on_battlefield(&engine, P0, forest())
        .into_iter()
        .find(|id| *id != land)
        .unwrap();
    assert!(!types(&engine, other).contains(TypeSet::CREATURE));
    reach_their_main_phase(&mut engine, lk::P1);
    assert!(types(&engine, land).contains(TypeSet::CREATURE));
    assert_eq!(pt(&engine, land), (2, 2));
}
