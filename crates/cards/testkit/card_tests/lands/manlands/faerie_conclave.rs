//! `cards/lands/manlands/faerie_conclave.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Faerie Conclave enters tapped, prints `{T}: Add {U}` and pays `{1}{U}` to
/// become a 2/1 blue Faerie with flying until end of turn — "It's still a
/// land". That last clause is the one that could go missing quietly, so the
/// board is read for `CREATURE` **and** `LAND` after the ability resolves,
/// and it is the land's own `{T}` that pays into the pool in the same breath
/// (three blue, two Islands and the Conclave) with the unanimated board
/// asserted as no creature above it. Entering tapped is played rather than
/// assumed: the land is played for real and a whole turn cycle is what shows
/// the untap step — not a turn that never came — is why it stands back up.
#[test]
fn faerie_conclave_enters_tapped_and_animates_into_a_flier_that_is_still_a_land() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(), island()])
        .hand(0, &[faerie_conclave()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let land = play_land(&mut engine, p0, faerie_conclave());
    assert!(entered_tapped(&engine, land), "\"This land enters tapped\"");
    assert!(
        !types(&engine, land).contains(TypeSet::CREATURE),
        "and it is a plain land until something is paid for — the control \
         for the animation below"
    );

    // A land that entered tapped gives nothing in the turn it arrived, so the
    // `{T}` below is read after its controller's next untap step.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert!(!is_tapped(&engine, land), "the untap step gave it back");

    // Both halves of the board tapped in one pass: the two Islands and the
    // Conclave's own printed `{T}: Add {U}`, which is the third blue.
    tap_all_mana(&mut engine, p0);
    assert!(is_tapped(&engine, land), "the {{T}} was the Conclave's own");
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Blue),
        3,
        "two Islands and the Conclave: its {{U}} is in the pool"
    );

    // {1}{U}: ability 1, behind the mana ability at index 0. The animation
    // costs no tap, which is why the land being tapped costs it nothing.
    activate(&mut engine, p0, faerie_conclave(), 1);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        1,
        "the animation's {{1}}{{U}} came out of the pool"
    );

    let kinds = types(&engine, land);
    assert!(
        kinds.contains(TypeSet::CREATURE),
        "it is a creature until end of turn: {kinds:?}"
    );
    assert!(
        kinds.contains(TypeSet::LAND),
        "\"It's still a land\": the animation adds a type and takes none \
         away: {kinds:?}"
    );
    assert_eq!(pt(&engine, land), (2, 1), "a 2/1 Faerie");
    assert!(
        keywords(&engine, land).contains(KeywordSet::FLYING),
        "with flying"
    );
}
