//! `cards/creatures/mv_6/canopy_dragon.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

const P0: PlayerId = PlayerId::new(0);

fn canopy_dragon() -> CardIndex {
    card_index("a9e59ee4-02a6-498c-82f0-f4f2ff334d65")
}

/// Canopy Dragon (a9e59ee4-02a6-498c-82f0-f4f2ff334d65) — {4}{G}{G} 4/4:
/// "Trample; {1}{G}: This creature gains flying and loses trample until end of turn."
#[test]
fn canopy_dragon_swaps_trample_for_flying_until_end_of_turn() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[canopy_dragon(), forest(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let dragon = on_battlefield(&engine, P0, canopy_dragon()).unwrap();
    let kw_before = keywords(&engine, dragon);
    assert!(kw_before.contains(KeywordSet::TRAMPLE));
    assert!(!kw_before.contains(KeywordSet::FLYING));

    tap_all_mana(&mut engine, P0);
    activate(&mut engine, P0, canopy_dragon(), 0);
    pass_until(&mut engine, stack_is_empty);

    let kw_after = keywords(&engine, dragon);
    assert!(kw_after.contains(KeywordSet::FLYING));
    assert!(!kw_after.contains(KeywordSet::TRAMPLE));
}

/// "until end of turn": on the opponent's next turn the Dragon tramples and
/// does not fly again.
#[test]
fn canopy_dragon_swap_ends_with_the_turn() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[canopy_dragon(), forest(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let dragon = on_battlefield(&engine, P0, canopy_dragon()).unwrap();
    tap_all_mana(&mut engine, P0);
    activate(&mut engine, P0, canopy_dragon(), 0);
    pass_until(&mut engine, stack_is_empty);
    assert!(keywords(&engine, dragon).contains(KeywordSet::FLYING));

    reach_their_main_phase(&mut engine, PlayerId::new(1));
    let after = keywords(&engine, dragon);
    assert!(!after.contains(KeywordSet::FLYING));
    assert!(after.contains(KeywordSet::TRAMPLE));
}
