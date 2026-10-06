//! `cards/creatures/mv_2/capashen_unicorn.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Capashen Unicorn` prints an activated ability with cost `{{1}}{{W}}, {{T}}, Sacrifice this creature`
/// to destroy target artifact or enchantment under `Coverage::Implemented`.
/// Activating the ability targeting an opponent's enchantment verifies that the Unicorn sacrifices itself,
/// pays its costs, and destroys the target enchantment upon resolution.
#[test]
fn capashen_unicorn_sacrifices_to_destroy_enchantment() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(1605, forest())
        .battlefield(0, &[capashen_unicorn(), plains(), plains()])
        .battlefield(1, &[their_enchantment()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let enchantment =
        on_battlefield(&engine, p1, their_enchantment()).expect("enchantment is seated");
    tap_all_mana(&mut engine, p0);

    activate(&mut engine, p0, capashen_unicorn(), 0);
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected target choice for Capashen Unicorn");
    };
    assert!(options.contains(&enchantment));
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![enchantment],
            },
        )
        .unwrap();

    pass_until(&mut engine, stack_is_empty);

    assert!(in_graveyard(&engine, p0, capashen_unicorn()).is_some());
    assert!(in_graveyard(&engine, p1, their_enchantment()).is_some());
    assert!(on_battlefield(&engine, p1, their_enchantment()).is_none());
}
