//! `cards/enchantments/auras/mv_2/consuming_ferocity.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::{CONSUMING_FEROCITY, GRIZZLY_BEARS};

const P0: PlayerId = PlayerId::new(0);

/// Consuming Ferocity (8efbe68c-622c-4986-a375-7e0bf780eb2e) — {1}{R} Aura:
/// "Enchant non-Wall creature. Enchanted creature gets +1/+0. At the beginning
/// of your upkeep, put a +1/+0 counter on enchanted creature. If that creature
/// has three or more +1/+0 counters on it, ... destroy that creature and it
/// can't be regenerated."
#[test]
fn consuming_ferocity_adds_p1p0_counters_and_destroys_at_three_counters() {
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[mountain(), mountain(), GRIZZLY_BEARS])
        .hand(0, &[CONSUMING_FEROCITY])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let bears = on_battlefield(&engine, P0, GRIZZLY_BEARS).unwrap();
    tap_all_mana(&mut engine, P0);
    cast_with_floating(&mut engine, P0, CONSUMING_FEROCITY);
    aim_at(&mut engine, P0, bears);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(pt(&engine, bears), (3, 2));

    // Upkeep 1 (turn 3): 1 +1/+0 counter -> 4/2.
    pass_until(&mut engine, |e| {
        e.state().turn.active == P0
            && e.state().turn.number == 3
            && e.state().turn.phase == Phase::FirstMain
    });
    assert_eq!(pt(&engine, bears), (4, 2));

    // Upkeep 2 (turn 5): 2 +1/+0 counters -> 5/2.
    pass_until(&mut engine, |e| {
        e.state().turn.active == P0
            && e.state().turn.number == 5
            && e.state().turn.phase == Phase::FirstMain
    });
    assert_eq!(pt(&engine, bears), (5, 2));

    // Upkeep 3 (turn 7): 3rd +1/+0 counter -> destroyed.
    pass_until(&mut engine, |e| {
        e.state().turn.active == P0
            && e.state().turn.number == 7
            && e.state().turn.phase == Phase::FirstMain
    });
    assert!(on_battlefield(&engine, P0, GRIZZLY_BEARS).is_none());
}
