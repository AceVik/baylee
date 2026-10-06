//! `cards/creatures/mv_3/storm_shaman.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Storm Shaman — {2}{R}, a printed 0/4 whose whole text is one line:
/// "{R}: This creature gets +1/+0 until end of turn."
///
/// Two Mountains are the entire cost of both activations and the pool is read
/// *after* they are tapped, because `legal.abilities` is filtered through
/// `can_afford`, which reads the pool and not the untapped lands — the pool
/// is empty once the second activation resolves, so `(2, 4)` can only have
/// been bought rather than assumed. The Raptor beside it is the control for
/// the word "This": a pump whose filter had widened would read the same in the
/// card file and a different number on this board, where the bystander stays a
/// printed 1/1. The walk into the opponent's turn is the "until end of turn",
/// read where it cannot be mistaken for a pump that never landed.
#[test]
fn storm_shaman_pays_one_red_to_pump_only_itself_until_end_of_turn() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[mountain(), mountain(), storm_shaman(), umara_raptor()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let shaman = on_battlefield(&engine, p0, storm_shaman()).expect("the Shaman is on the table");
    let bystander =
        on_battlefield(&engine, p0, umara_raptor()).expect("the Raptor is on the table");
    assert_eq!(pt(&engine, shaman), (0, 4), "the body the card prints");
    assert_eq!(
        pt(&engine, bystander),
        (1, 1),
        "and a creature the pump must not reach"
    );

    // Mana before the claim. Two Mountains are the only mana sources on this
    // board, so a pool of two is also the exact number of activations the
    // card's {R} can be paid for today.
    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        2,
        "two Mountains, two red, and nothing else that could have made mana"
    );
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(shaman, 0)),
        "with {{R}} floating, the only line the card prints is offered: {:?}",
        legal.abilities
    );

    activate(&mut engine, p0, storm_shaman(), 0);
    assert!(
        matches!(drive_to_rest(&mut engine, p0), Rest::Reached),
        "one activation resolves and the activating seat is back at a quiet priority"
    );
    assert_eq!(
        pt(&engine, shaman),
        (1, 4),
        "+1/+0 until end of turn: the power moves and the toughness does not"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        1,
        "the {{R}} came out of the pool the Mountains filled"
    );

    activate(&mut engine, p0, storm_shaman(), 0);
    assert!(
        matches!(drive_to_rest(&mut engine, p0), Rest::Reached),
        "a second activation is paid out of the same pool"
    );
    assert_eq!(
        pt(&engine, shaman),
        (2, 4),
        "two activations are two +1/+0s: the pump stacks instead of replacing itself"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "and both Mountains were spent on it"
    );
    assert_eq!(
        pt(&engine, bystander),
        (1, 1),
        "\"This creature\" — the pump lands on the Shaman and on no creature beside it"
    );

    // The other side of the same price: with the pool empty and both
    // Mountains tapped there is nothing left to pay it with, and the ability
    // is absent from the offer rather than listed and then refused.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        !legal.abilities.contains(&(shaman, 0)),
        "an unpayable {{R}} is not an option to take: {:?}",
        legal.abilities
    );

    reach_their_main_phase(&mut engine, p1);
    assert_eq!(
        pt(&engine, shaman),
        (0, 4),
        "\"until end of turn\": the pump was gone by the time the next turn began"
    );
}
