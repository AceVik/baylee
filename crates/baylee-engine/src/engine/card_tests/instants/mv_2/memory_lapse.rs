//! Memory Lapse — `{1}{U}` Instant (Homelands)
//!
//! Counter target spell. If that spell is countered this way, put it on top of
//! its owner's library instead of into that player's graveyard.
//!
//! (`Coverage::Partial`: putting a countered spell on top of its owner's
//! library instead of into their graveyard is not in `Effect`; counters target
//! spell into its owner's graveyard.)

#[allow(clippy::wildcard_imports)]
use super::*;

fn memory_lapse() -> CardIndex {
    card_index("bbfb3e4a-b389-4391-8141-13b68c0ef2e0")
}

fn bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

#[test]
fn memory_lapse_counters_target_spell() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, island())
        .hand(0, &[bears()])
        .battlefield(0, &[forest(), forest()])
        .hand(1, &[memory_lapse()])
        .battlefield(1, &[island(), island()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let bear = in_hand(&engine, p0, bears()).unwrap();
    let lapse = in_hand(&engine, p1, memory_lapse()).unwrap();

    tap_all_mana(&mut engine, p0);
    engine
        .apply(p0, PlayerAction::CastSpell { card: bear })
        .unwrap();
    let spell_on_stack = on_stack(&engine, bears()).unwrap();
    engine.apply(p0, PlayerAction::PassPriority).unwrap();

    tap_all_mana(&mut engine, p1);
    engine
        .apply(p1, PlayerAction::CastSpell { card: lapse })
        .unwrap();
    engine
        .apply(
            p1,
            PlayerAction::ChooseTargets {
                objects: vec![spell_on_stack],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p0, bears()).is_none());
    assert!(in_graveyard(&engine, p0, bears()).is_some());
}
