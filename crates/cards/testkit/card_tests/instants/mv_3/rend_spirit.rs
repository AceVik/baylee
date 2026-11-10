//! `cards/instants/mv_3/rend_spirit.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Rend Spirit` is an instant costing `{2}{B}` under `Coverage::Implemented`.
/// It prints "Destroy target Spirit."
/// When cast against an opponent's board containing both a Spirit (`Skyclave Apparition`) and a non-Spirit
/// (`Llanowar Elves`), targeting filters exclusively for Spirit creatures. Choosing the Spirit destroys it
/// upon resolution, leaving non-Spirit creatures on the battlefield.
#[test]
fn rend_spirit_destroys_target_spirit_only() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[swamp(), swamp(), swamp()])
        .hand(0, &[rend_spirit()])
        .battlefield(1, &[skyclave_apparition(), llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let spirit =
        on_battlefield(&engine, p1, skyclave_apparition()).expect("opponent controls Spirit");
    let elf = on_battlefield(&engine, p1, llanowar_elves()).expect("opponent controls non-Spirit");

    cast_from_hand(&mut engine, p0, rend_spirit());

    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!(
            "expected ChooseTargets prompt for Rend Spirit, got {:?}",
            engine.pending()
        );
    };
    assert!(
        options.contains(&spirit),
        "Spirit creature is a legal target: {options:?}"
    );
    assert!(
        !options.contains(&elf),
        "non-Spirit creature is excluded from target options: {options:?}"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![spirit],
            },
        )
        .expect("targeting Spirit is legal");

    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, p1, skyclave_apparition()).is_none(),
        "targeted Spirit was destroyed from the battlefield"
    );
    assert!(
        in_graveyard(&engine, p1, skyclave_apparition()).is_some(),
        "destroyed Spirit is in opponent's graveyard"
    );
    assert!(
        on_battlefield(&engine, p1, llanowar_elves()).is_some(),
        "non-Spirit creature remains on the battlefield"
    );
    assert!(
        in_graveyard(&engine, p0, rend_spirit()).is_some(),
        "resolved Rend Spirit sits in caster's graveyard"
    );
}
