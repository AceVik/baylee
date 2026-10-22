//! `cards/instants/mv_1/whispers_of_the_muse.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn whispers_of_the_muse() -> CardIndex {
    card_index("a4a8af4e-8686-45ba-94fd-ccf6cc8adaae")
}

/// `Whispers of the Muse` ({U}, Instant, `Coverage::Partial`):
/// `Buyback {5}`
/// `Draw a card.`
#[test]
fn whispers_of_the_muse_draws_a_card() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island()])
        .hand(0, &[whispers_of_the_muse()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();
    cast_from_hand(&mut engine, p0, whispers_of_the_muse());
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before
    );
}
