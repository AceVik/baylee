//! `cards/creatures/mv_7/krosan_groundshaker.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Krosan Groundshaker is a 6/6 Beast under `Coverage::Implemented` with an activated ability granting trample to a Beast.
/// Paying {G} targets a Beast creature and grants it trample until end of turn.
/// The targeting filter requires both creature and Beast types, excluding non-Beast creatures like Elves.
/// Resolving the ability grants `KeywordSet::TRAMPLE` to the target through the layer system while leaving bystanders unchanged.
#[test]
fn krosan_groundshaker_grants_trample_to_target_beast() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[krosan_groundshaker(), forest(), llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let krosan = on_battlefield(&engine, p0, krosan_groundshaker())
        .expect("Krosan Groundshaker is on battlefield");
    let elf =
        on_battlefield(&engine, p0, llanowar_elves()).expect("Llanowar Elves is on battlefield");

    assert_eq!(pt(&engine, krosan), (6, 6), "printed body is 6/6");
    assert!(
        !keywords(&engine, krosan).contains(KeywordSet::TRAMPLE),
        "starts without trample"
    );

    // The Elves are kept back: their own `{T}: Add {G}` is a second mana
    // route, and they are the bystander the target menu is read against.
    tap_all_mana_but(&mut engine, p0, Some(llanowar_elves()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        1,
        "one Forest provides one green mana"
    );

    activate(&mut engine, p0, krosan_groundshaker(), 0);

    let Pending::ChooseTargets {
        player,
        options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        panic!("expected target choice, got {:?}", engine.pending())
    };
    assert_eq!(player, p0, "activating player chooses target");
    assert_eq!((min, max), (1, 1), "exactly one target Beast required");
    assert!(
        options.contains(&krosan),
        "Krosan Groundshaker is a Beast and a legal target: {options:?}"
    );
    assert!(
        !options.contains(&elf),
        "Elf is not a Beast and cannot be targeted: {options:?}"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![krosan],
            },
        )
        .expect("targeting Krosan Groundshaker is legal");

    pass_until(&mut engine, stack_is_empty);

    assert!(
        keywords(&engine, krosan).contains(KeywordSet::TRAMPLE),
        "Krosan Groundshaker gained trample"
    );
    assert_eq!(pt(&engine, krosan), (6, 6), "body remains 6/6");
    assert!(
        !keywords(&engine, elf).contains(KeywordSet::TRAMPLE),
        "non-Beast creature gained no trample"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "green mana was spent"
    );
}
