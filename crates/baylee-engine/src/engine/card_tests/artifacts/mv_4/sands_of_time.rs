//! `cards/artifacts/mv_4/sands_of_time.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn sands_of_time() -> CardIndex {
    card_index("c8f69b48-a3b2-4324-852d-11fad19d6432")
}

/// `Sands of Time` ({4}, Artifact, `Coverage::Partial`) causes each player to
/// skip their untap step.
#[test]
fn sands_of_time_causes_players_to_skip_untap_step() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[plains(), plains(), plains(), plains()])
        .hand(0, &[sands_of_time()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = on_battlefield(&engine, p0, plains()).unwrap();
    cast_from_hand(&mut engine, p0, sands_of_time());
    pass_until(&mut engine, stack_is_empty);
    assert!(is_tapped(&engine, land));

    // Advance through P1's turn back to P0's main phase; P0 skips their untap step so the land stays tapped.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert!(is_tapped(&engine, land));
}
