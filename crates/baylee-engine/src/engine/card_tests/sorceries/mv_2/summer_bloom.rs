//! `cards/sorceries/mv_2/summer_bloom.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn summer_bloom() -> CardIndex {
    card_index("e5df4597-1647-4ac2-bdb3-a517598d1431")
}

/// `Summer Bloom` ({1}{G}, Sorcery, `Coverage::Implemented`) lets you play up
/// to three additional lands this turn.
#[test]
fn summer_bloom_allows_three_additional_land_plays_this_turn() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest()])
        .hand(
            0,
            &[
                summer_bloom(),
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, summer_bloom());
    pass_until(&mut engine, stack_is_empty);

    for _ in 0..4 {
        let land = in_hand(&engine, p0, forest()).unwrap();
        engine
            .apply(p0, PlayerAction::PlayLand { card: land })
            .expect("up to 4 lands allowed this turn");
    }
    let fifth = in_hand(&engine, p0, forest()).unwrap();
    assert!(
        engine
            .apply(p0, PlayerAction::PlayLand { card: fifth })
            .is_err()
    );
}
