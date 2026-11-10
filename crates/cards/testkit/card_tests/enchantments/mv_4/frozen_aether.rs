//! `cards/enchantments/mv_4/frozen_aether.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn frozen_aether() -> CardIndex {
    card_index("32452ccf-88c7-41d6-93e5-15d31f29726d")
}

/// "Artifacts, creatures, and lands your opponents control enter tapped.":
/// everything the opponent plays arrives tapped.
#[test]
fn frozen_aether_taps_what_the_opponent_puts_onto_the_battlefield() {
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[frozen_aether()])
        .hand(1, &[forest(), grizzly_bears(), sol_ring()])
        .start();
    keep_mulligans(&mut engine);
    reach_their_main_phase(&mut engine, p1);

    let land = play_land(&mut engine, p1, forest());
    assert!(is_tapped(&engine, land), "their land enters tapped");

    float(&mut engine, p1, ManaColor::Green, 2);
    cast_with_floating(&mut engine, p1, grizzly_bears());
    pass_until(&mut engine, stack_is_empty);
    let bears = on_battlefield(&engine, p1, grizzly_bears()).expect("resolved");
    assert!(is_tapped(&engine, bears), "their creature enters tapped");

    float(&mut engine, p1, ManaColor::Green, 1);
    cast_with_floating(&mut engine, p1, sol_ring());
    pass_until(&mut engine, stack_is_empty);
    let ring = on_battlefield(&engine, p1, sol_ring()).expect("resolved");
    assert!(is_tapped(&engine, ring), "their artifact enters tapped");
}

/// Mine are not "your opponents'": my land, creature and artifact enter
/// untapped.
#[test]
fn frozen_aether_leaves_my_own_permanents_untapped() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[frozen_aether()])
        .hand(0, &[forest(), grizzly_bears(), sol_ring()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, forest());
    assert!(!is_tapped(&engine, land), "my land enters untapped");

    float(&mut engine, p0, ManaColor::Green, 2);
    cast_with_floating(&mut engine, p0, grizzly_bears());
    pass_until(&mut engine, stack_is_empty);
    let bears = on_battlefield(&engine, p0, grizzly_bears()).expect("resolved");
    assert!(!is_tapped(&engine, bears), "my creature enters untapped");

    float(&mut engine, p0, ManaColor::Green, 1);
    cast_with_floating(&mut engine, p0, sol_ring());
    pass_until(&mut engine, stack_is_empty);
    let ring = on_battlefield(&engine, p0, sol_ring()).expect("resolved");
    assert!(!is_tapped(&engine, ring), "my artifact enters untapped");
}
