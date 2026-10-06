//! `cards/lands/utility/emeria_the_sky_ruin.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Emeria, the Sky Ruin prints `This land enters tapped.`, `At the beginning of your upkeep,
/// if you control seven or more Plains, you may return target creature card from your
/// graveyard to the battlefield.`, and `{{T}}: Add {{W}}.`
///
/// Marked `Coverage::Implemented`, all clauses are fully supported: `EnterModifier::Tapped`,
/// the upkeep trigger gated on seven controlled `plains()` returning a creature from the graveyard,
/// and the `{{T}}: Add {{W}}` mana ability. Playing Emeria enters tapped. On the subsequent upkeep
/// with seven Plains controlled, the trigger fires, asks for a target via `Pending::ChooseTargets`,
/// and returns the creature (`quiet_creature()`) from the graveyard to the battlefield.
/// Upon entering the main phase, Emeria stands untapped and taps for white mana.
#[test]
fn emeria_the_sky_ruin_enters_tapped_and_reanimates_at_upkeep_with_seven_plains() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, quiet_creature())
        .hand(0, &[emeria_the_sky_ruin()])
        .battlefield(
            0,
            &[
                plains(),
                plains(),
                plains(),
                plains(),
                plains(),
                plains(),
                plains(),
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    seed_graveyard(&mut engine, p0, 1);
    let creature = in_graveyard(&engine, p0, quiet_creature()).expect("creature in graveyard");

    let emeria = play_land(&mut engine, p0, emeria_the_sky_ruin());
    assert!(entered_tapped(&engine, emeria));

    // Pass turns until p0's upkeep trigger puts ChooseTargets on the stack.
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });

    let Pending::ChooseTargets {
        player,
        options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        panic!("expected ChooseTargets, got {:?}", engine.pending());
    };
    assert_eq!(player, p0);
    assert_eq!((min, max), (0, 1));
    assert!(options.contains(&creature));

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![creature],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, p0, quiet_creature()).is_some(),
        "creature returned to battlefield"
    );
    assert!(
        in_graveyard(&engine, p0, quiet_creature()).is_none(),
        "creature left the graveyard"
    );

    reach_main_phase(&mut engine, p0);
    assert!(!is_tapped(&engine, emeria));

    activate(&mut engine, p0, emeria_the_sky_ruin(), 0);
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::White), 1);
    assert!(is_tapped(&engine, emeria));
}
