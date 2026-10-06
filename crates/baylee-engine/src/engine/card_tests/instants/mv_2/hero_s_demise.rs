//! `cards/instants/mv_2/hero_s_demise.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Hero's Demise — {1}{B} Instant: "Destroy target legendary creature."
///
/// The whole card is one filter, so the scenario is built to tell
/// `LEGENDARY_CREATURE` from every wider reading it could have been: a
/// legendary creature stands across the table with a plain Llanowar Elves
/// beside it, and the target question must offer exactly the first. That the
/// legend is the *opponent's* and is still on the menu is the other half —
/// nothing on the card says "you control" — and the Elves standing unharmed
/// afterwards is the control that the destruction came from the spell and
/// not from a legend-rule accident or a board-wide wipe.
#[test]
fn heros_demise_destroys_the_legend_and_leaves_the_plain_creature_standing() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[swamp(), swamp()])
        .battlefield(1, &[katara_the_fearless(), llanowar_elves()])
        .hand(0, &[hero_s_demise()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let legend = on_battlefield(&engine, p1, katara_the_fearless())
        .expect("the legendary creature stands across the table");
    let elf =
        on_battlefield(&engine, p1, llanowar_elves()).expect("and a plain creature beside it");
    assert_eq!(
        pt(&engine, legend),
        (3, 3),
        "a body for the removal to be read off"
    );

    // Two Swamps for {1}{B}, and `cast_from_hand` taps them before the cast
    // so the offer is read against a pool that can actually pay.
    cast_from_hand(&mut engine, p0, hero_s_demise());
    let Pending::ChooseTargets {
        player,
        options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        panic!(
            "\"target legendary creature\" is a target choice, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the casting seat is the one that aims it");
    assert_eq!((min, max), (1, 1), "exactly one target");
    assert!(
        options.contains(&legend),
        "\"target legendary creature\" reaches across the table: {options:?}"
    );
    assert!(
        !options.contains(&elf),
        "a plain Llanowar Elves is a creature and no legend, so it is not on \
         the menu: {options:?}"
    );
    assert_eq!(
        options.len(),
        1,
        "and the legend is the whole of what the filter names on this board: {options:?}"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![legend],
            },
        )
        .expect("the creature the question offered is a legal target");
    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, p1, katara_the_fearless()).is_none(),
        "the targeted legend left the battlefield"
    );
    assert!(
        in_graveyard(&engine, p1, katara_the_fearless()).is_some(),
        "and it is in its owner's graveyard, which is where a destroyed \
         permanent goes"
    );
    assert!(
        on_battlefield(&engine, p1, llanowar_elves()).is_some(),
        "the creature the spell did not name never moved"
    );
    assert!(
        in_graveyard(&engine, p0, hero_s_demise()).is_some(),
        "the instant itself resolved and went to its caster's graveyard"
    );
}
