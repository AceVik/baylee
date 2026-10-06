//! `cards/enchantments/mv_3/energy_flux.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Energy Flux — {2}{U} — "All artifacts have 'At the beginning of your
/// upkeep, sacrifice this artifact unless you pay {2}.'"
///
/// The granted trigger belongs to the *artifact's* controller, not to Energy
/// Flux's (CR 113.7, CR 201.5b): p1's Sol Ring is asked about during p1's
/// upkeep and p0's Pendant during p0's, and each decline sends only that
/// artifact to its owner's graveyard while Energy Flux itself stays. One
/// artifact per side is the whole proof — a trigger collected once, for
/// Energy Flux's own controller, would ask p0 twice and never p1.
#[test]
fn energy_flux_taxes_each_artifacts_controller_in_their_own_upkeep() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[island(), island(), island(), darksteel_pendant()])
        .hand(0, &[energy_flux()])
        .battlefield(1, &[quiet_artifact(), island(), island()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    cast_from_hand(&mut engine, p0, energy_flux());
    pass_until(&mut engine, |e| {
        on_battlefield(e, p0, energy_flux()).is_some() && stack_is_empty(e)
    });

    // p1's turn comes first, so the first tax is the Sol Ring's controller's.
    pass_until(&mut engine, |e| {
        matches!(
            e.pending(),
            Pending::YesNo {
                prompt: YesNoPrompt::PayTax { .. },
                ..
            }
        )
    });
    let Pending::YesNo { player, prompt, .. } = engine.pending().clone() else {
        unreachable!("pass_until stopped on the question")
    };
    assert_eq!(engine.state().turn.active, p1, "p1's own upkeep");
    assert_eq!(player, p1, "\"your upkeep\" is the artifact's controller's");
    assert_eq!(
        prompt,
        YesNoPrompt::PayTax { mana: 2 },
        "\"unless you pay {{2}}\""
    );
    engine.apply(p1, PlayerAction::YesNo(false)).unwrap();
    assert!(
        in_graveyard(&engine, p1, quiet_artifact()).is_some(),
        "declining sacrifices the artifact the trigger was granted to"
    );
    assert!(
        on_battlefield(&engine, p0, energy_flux()).is_some(),
        "and never Energy Flux itself"
    );

    // The next tax is p0's own artifact's, two turns later.
    pass_until(&mut engine, |e| {
        matches!(
            e.pending(),
            Pending::YesNo {
                prompt: YesNoPrompt::PayTax { mana: 2 },
                player,
                ..
            } if *player == p0
        )
    });
    assert_eq!(engine.state().turn.active, p0, "p0's own upkeep");
    engine.apply(p0, PlayerAction::YesNo(false)).unwrap();
    assert!(
        in_graveyard(&engine, p0, darksteel_pendant()).is_some(),
        "the artifact under Energy Flux's controller is taxed too"
    );
}

/// The other answer the printed sentence offers: paying {2} keeps the
/// artifact. The two comes out of the controller's own pool, made on the spot
/// because the payment asked for it (CR 605.3a), and the third Island is the
/// change that says exactly two were spent.
#[test]
fn energy_flux_lets_an_artifact_pay_two_and_stay() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[island(), island(), island(), darksteel_pendant()])
        .hand(0, &[energy_flux()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    cast_from_hand(&mut engine, p0, energy_flux());
    pass_until(&mut engine, |e| {
        on_battlefield(e, p0, energy_flux()).is_some() && stack_is_empty(e)
    });

    pass_until(&mut engine, |e| {
        matches!(
            e.pending(),
            Pending::YesNo {
                prompt: YesNoPrompt::PayTax { mana: 2 },
                player,
                ..
            } if *player == p0
        )
    });
    engine.apply(p0, PlayerAction::YesNo(true)).unwrap();
    tap_all_mana(&mut engine, p0);
    engine.apply(p0, PlayerAction::PassPriority).unwrap();

    assert!(
        on_battlefield(&engine, p0, darksteel_pendant()).is_some(),
        "paying {{2}} keeps the artifact on the battlefield"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        1,
        "three Islands made three blue and the trigger spent exactly two"
    );
}
