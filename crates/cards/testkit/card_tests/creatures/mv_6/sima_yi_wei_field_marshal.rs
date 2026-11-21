//! `cards/creatures/mv_6/sima_yi_wei_field_marshal.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

const P0: PlayerId = PlayerId::new(0);

fn sima_yi_wei_field_marshal() -> CardIndex {
    card_index("eefa9a94-9915-4dcd-b5dc-7f8e9aa621a7")
}

/// Sima Yi, Wei Field Marshal (eefa9a94-9915-4dcd-b5dc-7f8e9aa621a7) — `{5}{B}` `*/4`
/// Legendary Creature — Human Soldier:
/// "Sima Yi, Wei Field Marshal's power is equal to the number of Swamps you control."
#[test]
fn sima_yi_wei_field_marshal_power_tracks_only_swamps_you_control() {
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(
            0,
            &[
                swamp(),
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                mountain(),
            ],
        )
        .battlefield(1, &[swamp(), swamp(), swamp()])
        .hand(0, &[sima_yi_wei_field_marshal(), badlands()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    // CDA applies off-battlefield as well: 1 Swamp controlled -> power 1 in hand.
    let in_hand_id = in_hand(&engine, P0, sima_yi_wei_field_marshal()).unwrap();
    assert_eq!(pt(&engine, in_hand_id), (1, 4));

    cast_from_hand(&mut engine, P0, sima_yi_wei_field_marshal());
    pass_until(&mut engine, stack_is_empty);

    let sima_yi =
        on_battlefield(&engine, P0, sima_yi_wei_field_marshal()).expect("Sima Yi entered");
    assert_eq!(
        pt(&engine, sima_yi),
        (1, 4),
        "only P0's 1 Swamp counts, not P1's 3 Swamps or P0's 5 Mountains"
    );

    // Playing Badlands (Mountain Swamp) raises Swamp count to 2.
    let bl = in_hand(&engine, P0, badlands()).unwrap();
    engine
        .apply(P0, PlayerAction::PlayLand { card: bl })
        .expect("play Badlands as land drop");
    assert_eq!(pt(&engine, sima_yi), (2, 4));
}

#[test]
fn sima_yi_with_zero_swamps_is_zero_four_and_survives() {
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[sima_yi_wei_field_marshal(), mountain()])
        .battlefield(1, &[swamp(), swamp()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let sima_yi = on_battlefield(&engine, P0, sima_yi_wei_field_marshal()).unwrap();
    assert_eq!(pt(&engine, sima_yi), (0, 4));
}
