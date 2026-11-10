//! `cards/creatures/mv_6/gruul_nodorog.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Gruul Nodorog` is a 4/4 Beast costing `{4}{G}{G}` under `Coverage::Implemented`.
/// It prints "{R}: This creature gains menace until end of turn."
/// When activated off floating red mana, its ability resolves and grants `KeywordSet::MENACE`
/// to the creature until end of turn.
#[test]
fn gruul_nodorog_gains_menace_until_end_of_turn() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[gruul_nodorog(), mountain()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let nodorog = on_battlefield(&engine, p0, gruul_nodorog()).expect("Gruul Nodorog is present");
    assert_eq!(pt(&engine, nodorog), (4, 4), "printed body is 4/4");
    assert!(
        !keywords(&engine, nodorog).contains(KeywordSet::MENACE),
        "Gruul Nodorog does not have menace initially"
    );

    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Red),
        1,
        "one red mana floated"
    );

    activate(&mut engine, p0, gruul_nodorog(), 0);
    pass_until(&mut engine, stack_is_empty);

    assert!(
        keywords(&engine, nodorog).contains(KeywordSet::MENACE),
        "Gruul Nodorog gained menace until end of turn"
    );
}
