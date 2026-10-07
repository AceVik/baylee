//! `cards/instants/mv_1/burst_of_energy.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Burst of Energy — {W} instant: "Untap target permanent." The word worth
/// playing is **permanent**, because the target spec is `Filter::Any`: anything
/// on either battlefield, not merely a creature of your own. So the board
/// carries two Plains under p0 — one of them tapped to pay the {W} — and a
/// Forest across the table, and the offer has to name all three. The untap
/// itself is then read off the board rather than off the question: the Plains
/// the spell named stands back up and the Plains nobody named stays down.
/// Both of them are tapped first, which is what makes the second half a
/// claim at all: a land that was never tapped is standing afterwards whether
/// or not the spell did anything.
#[test]
fn burst_of_energy_untaps_the_permanent_it_names_and_no_other() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[plains(), plains()])
        .battlefield(1, &[forest()])
        .hand(0, &[burst_of_energy()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let lands = all_on_battlefield(&engine, p0, plains());
    assert_eq!(lands.len(), 2, "two Plains were dealt");
    let (spent, kept) = (lands[0], lands[1]);
    let theirs = on_battlefield(&engine, p1, forest()).expect("a Forest across the table");

    // **Both** Plains are tapped, and that is the whole control. A Plains
    // that was never tapped is untapped afterwards whatever the spell did,
    // so "and no other" would have been satisfied by a spell that did
    // nothing at all. Two down, one named, one not.
    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        2,
        "two white floating, one of which is the {{W}} the spell prints"
    );
    assert!(
        is_tapped(&engine, spent) && is_tapped(&engine, kept),
        "both Plains paid for it and both are down"
    );

    cast_with_floating(&mut engine, p0, burst_of_energy());
    let Pending::ChooseTargets {
        player,
        options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        panic!(
            "\"target permanent\" is a target choice, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the seat that cast it aims it");
    assert_eq!((min, max), (1, 1), "one permanent, no more and no fewer");
    assert!(
        options.contains(&spent),
        "the tapped Plains is a permanent: {options:?}"
    );
    assert!(
        options.contains(&kept),
        "and so is the untapped one: {options:?}"
    );
    assert!(
        options.contains(&theirs),
        "\"target permanent\" reaches across the table: {options:?}"
    );
    assert!(
        is_tapped(&engine, spent),
        "`CR 601.2h`: nothing is untapped while the target question still stands"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![spent],
            },
        )
        .expect("the permanent the question offered was chosen");
    pass_until(&mut engine, stack_is_empty);

    assert!(
        !is_tapped(&engine, spent),
        "\"Untap target permanent\" — the permanent the spell named is standing"
    );
    assert!(
        is_tapped(&engine, kept),
        "and the one nobody named is still the tapped Plains it was"
    );
    assert!(
        !is_tapped(&engine, theirs),
        "the Forest across the table was never tapped and still is not"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        1,
        "one {{W}} paid for the spell and the other is still floating — the \
         phase has not ended (CR 500.4)"
    );
    assert!(
        in_graveyard(&engine, p0, burst_of_energy()).is_some(),
        "an instant resolves into its owner's graveyard"
    );
}
