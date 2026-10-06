//! `cards/lands/utility/tower_of_the_magistrate.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Tower of the Magistrate` is a utility land under `Coverage::Implemented`.
/// It prints "{T}: Add {C}." and "{1}, {T}: Target creature gains protection from artifacts until end of turn."
/// By reserving `Tower of the Magistrate` and tapping another source for `{1}`, its second ability
/// targets a creature and grants it `Modifier::ProtectionFrom(&Filter::ARTIFACT)`.
#[test]
fn tower_of_the_magistrate_grants_protection_from_artifacts_to_target_creature() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[tower_of_the_magistrate(), forest(), llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let elf = on_battlefield(&engine, p0, llanowar_elves()).expect("elf deployed");

    tap_all_mana_but(&mut engine, p0, Some(tower_of_the_magistrate()));
    activate(&mut engine, p0, tower_of_the_magistrate(), 1);

    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!(
            "expected ChooseTargets prompt for Tower of the Magistrate, got {:?}",
            engine.pending()
        );
    };
    assert!(
        options.contains(&elf),
        "target creature is an option: {options:?}"
    );

    engine
        .apply(p0, PlayerAction::ChooseObjects { objects: vec![elf] })
        .unwrap();

    pass_until(&mut engine, stack_is_empty);

    let tower = on_battlefield(&engine, p0, tower_of_the_magistrate())
        .expect("Tower of the Magistrate is on the battlefield");
    assert!(
        is_tapped(&engine, tower),
        "Tower of the Magistrate is tapped"
    );

    assert!(
        engine.state().effects.iter().any(|fx| {
            matches!(
                fx.modifier,
                baylee_cards_dsl::Modifier::ProtectionFrom(&baylee_cards_dsl::Filter::ARTIFACT)
            )
        }),
        "continuous effect granting protection from artifacts is active"
    );
}
