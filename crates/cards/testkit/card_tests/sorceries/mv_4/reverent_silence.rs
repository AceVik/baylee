//! `cards/sorceries/mv_4/reverent_silence.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

#[test]
fn reverent_silence_destroys_all_enchantments() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[card_index("e670d1c9-5e28-4df8-aad9-d08d19dc74e9")])
        .battlefield(
            0,
            &[
                forest(),
                forest(),
                forest(),
                forest(),
                index::SAPROLING_CLUSTER,
                index::GRIZZLY_BEARS,
            ],
        )
        .battlefield(1, &[index::NOBLE_STAND])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(
        &mut engine,
        p0,
        card_index("e670d1c9-5e28-4df8-aad9-d08d19dc74e9"),
    );
    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, p0, index::SAPROLING_CLUSTER).is_none(),
        "friendly enchantment destroyed"
    );
    assert!(
        on_battlefield(&engine, p1, index::NOBLE_STAND).is_none(),
        "opponent enchantment destroyed"
    );
    assert!(
        on_battlefield(&engine, p0, index::GRIZZLY_BEARS).is_some(),
        "non-enchantments untouched"
    );
}
