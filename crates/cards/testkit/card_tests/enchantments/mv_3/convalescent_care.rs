//! `cards/enchantments/mv_3/convalescent_care.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

#[test]
fn convalescent_care_triggers_at_upkeep_only_when_at_five_or_less_life() {
    let p0 = PlayerId::new(0);
    let care = card_index("162407d9-3e3c-440f-a7a9-22936d99fc8a");
    let mut engine = Duel::new(SEED, plains())
        .life(0, 5)
        .battlefield(0, &[care])
        .start();
    keep_mulligans(&mut engine);
    let lib_before = library_size(&engine, p0);
    reach_their_main_phase(&mut engine, p0);

    assert_eq!(engine.state().players[0].life, 8);
    // Starting player skips turn-1 draw step (CR 103.8a), so only Convalescent Care draws 1 card.
    assert_eq!(library_size(&engine, p0), lib_before - 1);
}
