//! `cards/creatures/mv_1/akki_avalanchers.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Akki Avalanchers` prints `Sacrifice a land: This creature gets +2/+0 until end of turn. Activate only once each turn.` with `Coverage::Implemented`.
/// In this scenario, seat 0 controls the 1/1 creature and one Mountain.
/// Activating the ability prompts with `ChoicePrompt::CostSacrifice` to sacrifice the land.
/// Upon resolution, `Akki Avalanchers` becomes a 3/1 creature and its per-turn activation limit prevents further activations.
#[test]
fn akki_avalanchers_sacrifices_land_to_pump_power_once_per_turn() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[akki_avalanchers(), mountain()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let akki = on_battlefield(&engine, p0, akki_avalanchers()).expect("akki seated");
    let land = on_battlefield(&engine, p0, mountain()).expect("mountain seated");
    assert_eq!(pt(&engine, akki), (1, 1));

    activate(&mut engine, p0, akki_avalanchers(), 0);
    let Pending::ChooseCards {
        player,
        options,
        prompt,
        ..
    } = engine.pending().clone()
    else {
        panic!("expected CostSacrifice prompt, got {:?}", engine.pending());
    };
    assert_eq!(prompt, ChoicePrompt::CostSacrifice);
    assert!(options.contains(&land), "mountain is offered as sacrifice");

    engine
        .apply(
            player,
            PlayerAction::ChooseObjects {
                objects: vec![land],
            },
        )
        .expect("sacrifice pays the cost");

    assert!(
        in_graveyard(&engine, p0, mountain()).is_some(),
        "mountain was sacrificed"
    );

    pass_until(&mut engine, stack_is_empty);
    assert_eq!(pt(&engine, akki), (3, 1), "gained +2/+0 until end of turn");

    // The ability has ActivationLimit::PerTurn(1), so it cannot be activated again.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending());
    };
    assert!(
        !legal
            .abilities
            .iter()
            .any(|(id, index)| *id == akki && *index == 0),
        "ability cannot be activated more than once per turn"
    );
}
