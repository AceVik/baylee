//! `cards/sorceries/mv_1/seething_anger.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::partials_w2c_kit::power_toughness;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);

fn seething_anger() -> CardIndex {
    card_index("2effcd1c-5833-4e0e-bb67-2d2ffb22a014")
}

/// Seething Anger (`2effcd1c-5833-4e0e-bb67-2d2ffb22a014`) — {R} Sorcery (Partial):
/// "Target creature gets +3/+0 until end of turn."
#[test]
fn seething_anger_gives_target_creature_plus_three_plus_zero() {
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[index::GRIZZLY_BEARS, mountain()])
        .hand(0, &[seething_anger()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let bear = on_battlefield(&engine, P0, index::GRIZZLY_BEARS).unwrap();
    cast_from_hand(&mut engine, P0, seething_anger());
    aim_at(&mut engine, P0, bear);
    pass_until(&mut engine, stack_is_empty);
    assert!(
        in_graveyard(&engine, P0, seething_anger()).is_some(),
        "base spell does not get free buyback"
    );

    assert_eq!(power_toughness(&engine, bear), (5, 2));
    reach_their_main_phase(&mut engine, PlayerId::new(1));
    assert_eq!(power_toughness(&engine, bear), (2, 2));
}
