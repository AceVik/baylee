//! `cards/artifacts/mv_4/pentagram_of_the_ages.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn pentagram_of_the_ages() -> CardIndex {
    card_index("265204a9-0ce2-4a0a-a9ea-8537cd94b079")
}

/// "{4}, {T}: The next time a source of your choice would deal damage to you
/// this turn, prevent that damage."
#[test]
fn pentagram_prevents_the_next_damage_from_the_chosen_source_to_you() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                pentagram_of_the_ages(),
                forest(),
                forest(),
                forest(),
                forest(),
                mountain(),
                mountain(),
            ],
        )
        .hand(0, &[lightning_bolt(), lightning_bolt()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let bolts = engine.state().zones.list(ZoneLocation::Hand(p0)).clone();
    let chosen_bolt = bolts[0];
    tap_all_mana(&mut engine, p0);
    engine
        .apply(p0, PlayerAction::CastSpell { card: chosen_bolt })
        .unwrap();
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p0],
            },
        )
        .unwrap();
    let stack_bolt = *engine.state().zones.list(ZoneLocation::Stack).last().unwrap();
    activate(&mut engine, p0, pentagram_of_the_ages(), 0);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseDamageSource { .. })
    });
    let Pending::ChooseDamageSource { choice, options, .. } = engine.pending().clone() else {
        unreachable!()
    };
    let source_ref = *options
        .iter()
        .find(|r| r.object == stack_bolt)
        .expect("the resolving Lightning Bolt is a legal source choice");
    engine
        .apply(
            p0,
            PlayerAction::ChooseDamageSource {
                choice,
                source: source_ref,
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(life_of(&engine, p0), 20, "the chosen source's damage was prevented");
}
