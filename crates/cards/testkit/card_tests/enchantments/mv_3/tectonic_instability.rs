//! `cards/enchantments/mv_3/tectonic_instability.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn tectonic_instability() -> CardIndex {
    card_index("0e68ee36-9649-490a-8483-fc8bdc23f161")
}

fn mountain() -> CardIndex {
    card_index("a3fb7228-e76b-4e96-a40e-20b5fed75685")
}

/// Tectonic Instability ({2}{R} Enchantment):
/// "Whenever a land enters, tap all lands its controller controls."
#[test]
fn tectonic_instability_taps_all_lands_controlled_by_entering_lands_controller() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[mountain(), mountain(), mountain(), plains()])
        .battlefield(1, &[island(), island()])
        .hand(0, &[tectonic_instability(), forest()])
        .hand(1, &[forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    tap_all_mana_but(&mut engine, P0, Some(plains()));
    cast_with_floating(&mut engine, P0, tectonic_instability());
    pass_until(&mut engine, stack_is_empty);

    // P0 plays Forest -> P0's Plains is tapped too, while P1's Islands remain untapped.
    let my_plains = on_battlefield(&engine, P0, plains()).unwrap();
    let their_island = on_battlefield(&engine, P1, island()).unwrap();
    play_land(&mut engine, P0, forest());
    pass_until(&mut engine, stack_is_empty);
    assert!(lk::tapped(&engine, my_plains));
    assert!(!lk::tapped(&engine, their_island));

    // P1 plays Forest -> all P1's lands tap.
    reach_their_main_phase(&mut engine, P1);
    play_land(&mut engine, P1, forest());
    pass_until(&mut engine, stack_is_empty);
    let their_forest = on_battlefield(&engine, P1, forest()).unwrap();
    assert!(lk::tapped(&engine, their_island));
    assert!(lk::tapped(&engine, their_forest));
}
