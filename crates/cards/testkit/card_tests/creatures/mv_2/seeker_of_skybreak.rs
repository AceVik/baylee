//! `cards/creatures/mv_2/seeker_of_skybreak.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Seeker of Skybreak is a {1}{G} 2/1 Elf printing "{T}: Untap target
/// creature." — the whole price is the tap symbol, so the ability is read off
/// a *tapped* creature: the Llanowar Elves paid for the Seeker's own `{1}{G}`
/// through `tap_all_mana` (#159), which leaves it face down and makes the
/// untap a change of state rather than a no-op. The Elf across the table is
/// the control for the target line — the card says "target creature" and not
/// "you control" — so it has to be offered and has to be the one that does
/// not move, which is what says the ability hit the creature that was named.
#[test]
fn seeker_of_skybreak_taps_to_untap_the_creature_it_names() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(41, forest())
        // Seated and not cast: the printed cost is `{T}`, which a creature
        // that arrived this turn cannot pay (CR 302.6), and what this test is
        // about is the untap rather than the casting.
        .battlefield(
            0,
            &[forest(), forest(), llanowar_elves(), seeker_of_skybreak()],
        )
        .battlefield(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let elves = on_battlefield(&engine, p0, llanowar_elves()).expect("my Elves are out");
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elves are out");

    // The Elf and nothing else: it is a mana route, so tapping it for its own
    // green makes it exactly the tapped permanent the printed ability is
    // about, while the two Forests stay standing and out of the way.
    let seeker = on_battlefield(&engine, p0, seeker_of_skybreak()).expect("the Seeker is seated");
    assert_eq!(pt(&engine, seeker), (2, 1), "the body the card prints");
    tap_mana_where(&mut engine, p0, |id| id == elves);
    assert!(
        is_tapped(&engine, elves),
        "the Elf made its green and is lying down"
    );
    assert!(
        !is_tapped(&engine, seeker),
        "and the Seeker stands up for it, so its {{T}} is still there"
    );

    // A printed `{T}` and nothing else: the ability is offered with an empty
    // pool and presses by index, which is the only index the card has.
    activate(&mut engine, p0, seeker_of_skybreak(), 0);
    let Pending::ChooseTargets {
        player, options, ..
    } = engine.pending().clone()
    else {
        panic!(
            "\"target creature\" is a target choice, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the activating seat is the one that chooses");
    assert!(
        options.contains(&elves),
        "the tapped creature is a legal target: {options:?}"
    );
    assert!(
        options.contains(&theirs),
        "\"target creature\" is not \"creature you control\", so the Elf \
         across the table is on the same menu: {options:?}"
    );
    // CR 601.2c names the target and CR 601.2h pays afterwards, so the Seeker
    // is still upright while the question stands.
    assert!(
        !is_tapped(&engine, seeker),
        "the cost is the last step of the activation, not the first"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![elves],
            },
        )
        .expect("the creature the question offered was chosen");
    assert!(is_tapped(&engine, seeker), "{{T}} was the whole price");
    pass_until(&mut engine, stack_is_empty);

    assert!(
        !is_tapped(&engine, elves),
        "\"Untap target creature\": the creature that was down is standing again"
    );
    assert!(
        !is_tapped(&engine, theirs),
        "and the creature the ability did not name never moved"
    );
    assert_eq!(
        pt(&engine, elves),
        (1, 1),
        "untapping is all it does: no counter, no pump, the printed 1/1"
    );
    assert!(
        on_battlefield(&engine, p0, seeker_of_skybreak()).is_some(),
        "an activated ability costs the Seeker nothing but its tap"
    );
}
