//! `cards/lands/utility/wirewood_lodge.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Wirewood Lodge` is a utility land under `Coverage::Implemented`.
/// It prints "{T}: Add {C}" and "{G}, {T}: Untap target Elf."
/// When an Elf (such as `Llanowar Elves`) taps to produce `{G}`, that green mana can be spent
/// to activate ability 1 of `Wirewood Lodge`, targeting the tapped Elf and untapping it upon resolution.
#[test]
fn wirewood_lodge_untaps_target_elf() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[wirewood_lodge(), llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let lodge = on_battlefield(&engine, p0, wirewood_lodge()).expect("Wirewood Lodge is present");
    let elf = on_battlefield(&engine, p0, llanowar_elves()).expect("Llanowar Elves is present");
    assert!(!is_tapped(&engine, lodge), "Wirewood Lodge starts untapped");
    assert!(!is_tapped(&engine, elf), "Elf starts untapped");

    // Tap the Elf for {G} to provide mana and put a tapped Elf on the battlefield.
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: elf,
                ability_index: 0,
            },
        )
        .expect("Elf taps for green mana");
    assert!(is_tapped(&engine, elf), "Elf is now tapped");
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Green),
        1,
        "one green mana in pool"
    );

    // Ability 0 is "{T}: Add {C}", ability 1 is "{G}, {T}: Untap target Elf".
    activate(&mut engine, p0, wirewood_lodge(), 1);

    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!(
            "expected ChooseTargets prompt for Wirewood Lodge, got {:?}",
            engine.pending()
        );
    };
    assert!(
        options.contains(&elf),
        "tapped Elf is a legal target: {options:?}"
    );

    engine
        .apply(p0, PlayerAction::ChooseObjects { objects: vec![elf] })
        .expect("targeting Elf is legal");

    pass_until(&mut engine, stack_is_empty);

    assert!(
        !is_tapped(&engine, elf),
        "Elf was untapped by Wirewood Lodge"
    );
    assert!(is_tapped(&engine, lodge), "Wirewood Lodge tapped as cost");
}
