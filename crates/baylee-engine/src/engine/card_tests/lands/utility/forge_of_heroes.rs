//! `cards/lands/utility/forge_of_heroes.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Forge of Heroes prints two lines and writes one: `{T}: Add {C}`, while the
/// commander line ("choose target commander that entered this turn", then a
/// +1/+1 or loyalty counter) is the `Coverage::Partial` gap with no DSL shape,
/// so the half that exists is the half played. The scenario runs it end to
/// end: the land is dropped with a real `PlayLand` — standing, because the
/// card prints no enters-tapped clause — tapped for its own mana, and the {C}
/// is then actually *spent*, a {1} Sol Ring cast out of the pool, which is what
/// separates mana that is usable from a number that only moved. The colour is
/// the load-bearing assertion: the whole backing deck is Forests, so green in
/// the pool would be the filler's mana wearing the Forge's name, and `{C}` is
/// colourless by definition (CR 105.4).
#[test]
fn forge_of_heroes_taps_for_colourless_mana_that_pays_for_a_one_cost_spell() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[forge_of_heroes(), quiet_artifact()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let land = play_land(&mut engine, p0, forge_of_heroes());
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "nothing floats before the Forge is tapped"
    );
    assert!(
        !is_tapped(&engine, land),
        "the card prints no enters-tapped clause, so the land drop leaves it standing"
    );

    let taken = tap_all_mana(&mut engine, p0);
    assert_eq!(
        taken, 1,
        "the Forge is the only mana source on this board — and a nonbasic land \
         printing its own `{{T}}: Add {{C}}` is a route the kit has to find (#159)"
    );
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Colorless),
        1,
        "`{{T}}: Add {{C}}`"
    );
    assert_eq!(pool.total(), 1, "one mana, off one tap");
    assert_eq!(
        pool.available(ManaColor::Green),
        0,
        "and not the Forests the library is made of"
    );
    assert!(is_tapped(&engine, land), "the Forge paid its own {{T}}");
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack, so nothing is waiting to resolve"
    );

    // The half a bare pool count cannot show: the {C} is spent.
    cast_with_floating(&mut engine, p0, quiet_artifact());
    pass_until(&mut engine, stack_is_empty);
    assert!(
        on_battlefield(&engine, p0, quiet_artifact()).is_some(),
        "the {{C}} paid a {{1}} cost, so what the Forge made was usable \
         colourless mana and not a colour it does not print"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "and the pool is empty because the spell took it"
    );
    assert!(
        on_battlefield(&engine, p0, forge_of_heroes()).is_some(),
        "the land itself is untouched by the tap that paid for the artifact"
    );
}
