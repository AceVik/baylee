//! `cards/creatures/mv_2/heartwood_dryad.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn heartwood_dryad() -> CardIndex {
    card_index("976968c5-7ef4-4451-86ff-03a011bbcca7")
}

/// Heartwood Dryad (`976968c5-7ef4-4451-86ff-03a011bbcca7`) — {1}{G} Creature — Dryad (Partial):
/// "This creature can block creatures with shadow as though it had shadow."
#[test]
fn heartwood_dryad_gains_shadow_on_opponents_turn_to_block_shadow_creatures() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[heartwood_dryad()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let dryad = on_battlefield(&engine, P0, heartwood_dryad()).unwrap();
    assert!(!keywords(&engine, dryad).contains(KeywordSet::SHADOW));

    reach_their_main_phase(&mut engine, P1);
    assert!(keywords(&engine, dryad).contains(KeywordSet::SHADOW));
}
