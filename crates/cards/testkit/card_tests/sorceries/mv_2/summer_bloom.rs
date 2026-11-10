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

/// Whether `seat` may play a land from hand right now (one `PlayLand` is made
/// when it may, so the caller counts the plays).
fn play_a_land(engine: &mut Engine<RegistryLookup>, seat: PlayerId) -> bool {
    let land = in_hand(engine, seat, forest()).expect("a Forest is in hand");
    engine
        .apply(seat, PlayerAction::PlayLand { card: land })
        .is_ok()
}

/// The three plays are *additional* ones, so a land played before the spell
/// leaves exactly three more, and the permission ends with the turn: the next
/// turn is back to the one land a turn (CR 305.2).
#[test]
fn summer_bloom_after_a_land_drop_adds_three_and_ends_with_the_turn() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
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
                forest(),
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    assert!(play_a_land(&mut engine, p0), "the turn's own land drop");
    cast_from_hand(&mut engine, p0, summer_bloom());
    pass_until(&mut engine, stack_is_empty);

    for n in 1..=3 {
        assert!(play_a_land(&mut engine, p0), "additional land {n} of 3");
    }
    assert!(
        !play_a_land(&mut engine, p0),
        "1 normal + 3 additional is the whole allowance, not 1 + 3 more"
    );

    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert!(
        play_a_land(&mut engine, p0),
        "the next turn has its own drop"
    );
    assert!(
        !play_a_land(&mut engine, p0),
        "Summer Bloom ended with the turn it was cast in"
    );
}
