//! `cards/enchantments/auras/mv_6/caustic_tar.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;

fn caustic_tar() -> CardIndex {
    card_index("2b5f70ec-33da-4dff-9cff-fe05eb906685")
}

/// Enchanted land has "{T}: Target player loses 3 life" (verified on opponent
/// and self, and on an enchanted opponent's land).
#[test]
fn caustic_tar_grants_tap_target_player_loses_three_life() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = lk::game(
        SEED,
        &[swamp()],
        &[plains()],
        &[caustic_tar(), caustic_tar()],
        &[],
    );
    let my_swamp = lk::obj(&e, p0, swamp());
    let opp_plains = lk::obj(&e, p1, plains());

    lk::float(
        &mut e,
        p0,
        &[(ManaColor::Black, 2), (ManaColor::Colorless, 4)],
    );
    lk::cast_at(&mut e, p0, caustic_tar(), &[my_swamp], &[]);
    lk::float(
        &mut e,
        p0,
        &[(ManaColor::Black, 2), (ManaColor::Colorless, 4)],
    );
    lk::cast_at(&mut e, p0, caustic_tar(), &[opp_plains], &[]);

    assert!(lk::offered(
        &e,
        swamp(),
        p0,
        baylee_engine::choice::GRANTED_ABILITY
    ));
    lk::use_ability(
        &mut e,
        p0,
        swamp(),
        baylee_engine::choice::GRANTED_ABILITY,
        &[],
        &[p1],
    );
    assert_eq!(lk::life(&e, p1), 17);

    // Opponent activates on their enchanted Plains targeting P0.
    reach_their_main_phase(&mut e, p1);
    lk::use_ability(
        &mut e,
        p1,
        plains(),
        baylee_engine::choice::granted_ability(1),
        &[],
        &[p0],
    );
    assert_eq!(lk::life(&e, p0), 17);
}
