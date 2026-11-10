//! `cards/enchantments/auras/mv_2/consuming_ferocity.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::GRIZZLY_BEARS;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn consuming_ferocity() -> CardIndex {
    card_index("8efbe68c-622c-4986-a375-7e0bf780eb2e")
}

/// Runs `turn` (an odd turn number: P0's) to its first main phase.
fn to_p0_main_of_turn(engine: &mut Engine<RegistryLookup>, turn: u32) {
    pass_until(engine, |e| {
        e.state().turn.active == P0
            && e.state().turn.number == turn
            && e.state().turn.phase == Phase::FirstMain
    });
}

/// Consuming Ferocity (8efbe68c-622c-4986-a375-7e0bf780eb2e) — {1}{R} Aura:
/// "Enchant non-Wall creature. Enchanted creature gets +1/+0. At the beginning
/// of your upkeep, put a +1/+0 counter on enchanted creature. If that creature
/// has three or more +1/+0 counters on it, it deals damage equal to its power
/// to its controller, then destroy that creature and it can't be regenerated."
///
/// On my own Bears (2/2, 3/2 with the Aura): one counter at each of my upkeeps,
/// nothing else happens at one and two, and at the third the Bears (6/2 by then)
/// deal 6 to their controller - me - and are destroyed.
#[test]
fn consuming_ferocity_deals_its_power_to_its_controller_at_three_counters() {
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[mountain(), mountain(), GRIZZLY_BEARS])
        .hand(0, &[consuming_ferocity()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let bears = on_battlefield(&engine, P0, GRIZZLY_BEARS).unwrap();
    tap_all_mana(&mut engine, P0);
    cast_with_floating(&mut engine, P0, consuming_ferocity());
    aim_at(&mut engine, P0, bears);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(pt(&engine, bears), (3, 2));

    // Upkeep 1 (turn 3): 1 +1/+0 counter -> 4/2. No damage.
    to_p0_main_of_turn(&mut engine, 3);
    assert_eq!(pt(&engine, bears), (4, 2));
    assert_eq!(engine.state().players[0].life, 20);

    // Upkeep 2 (turn 5): 2 +1/+0 counters -> 5/2. No damage.
    to_p0_main_of_turn(&mut engine, 5);
    assert_eq!(pt(&engine, bears), (5, 2));
    assert_eq!(engine.state().players[0].life, 20);

    // Upkeep 3 (turn 7): the 3rd counter -> 6/2, which deals 6 to its
    // controller, then it is destroyed.
    to_p0_main_of_turn(&mut engine, 7);
    assert!(on_battlefield(&engine, P0, GRIZZLY_BEARS).is_none());
    assert_eq!(
        engine.state().players[0].life,
        14,
        "the creature deals damage equal to its power (6) to its controller"
    );
    assert_eq!(engine.state().players[1].life, 20);
}

/// "To its controller" is the enchanted creature's controller, not the Aura's:
/// the Aura on an opponent's Bears drains the opponent, and I take nothing.
#[test]
fn consuming_ferocity_on_their_creature_damages_their_controller_not_mine() {
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[mountain(), mountain()])
        .battlefield(1, &[GRIZZLY_BEARS])
        .hand(0, &[consuming_ferocity()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let bears = on_battlefield(&engine, P1, GRIZZLY_BEARS).unwrap();
    tap_all_mana(&mut engine, P0);
    cast_with_floating(&mut engine, P0, consuming_ferocity());
    aim_at(&mut engine, P0, bears);
    pass_until(&mut engine, stack_is_empty);

    // One turn at a time: `pass_until` gives up after 100 passes, which is
    // fewer than the six turns to the third counter take in one run.
    to_p0_main_of_turn(&mut engine, 3);
    to_p0_main_of_turn(&mut engine, 5);
    to_p0_main_of_turn(&mut engine, 7);
    assert!(on_battlefield(&engine, P1, GRIZZLY_BEARS).is_none());
    assert_eq!(
        engine.state().players[1].life,
        14,
        "their creature, their life"
    );
    assert_eq!(engine.state().players[0].life, 20, "mine is untouched");
}
