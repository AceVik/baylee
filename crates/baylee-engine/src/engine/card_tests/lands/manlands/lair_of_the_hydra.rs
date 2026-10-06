//! `cards/lands/manlands/lair_of_the_hydra.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Lair of the Hydra: "If you control two or more other lands, this land
/// enters tapped."
///
/// The land was written `at_most: 2` where the four cards printing that
/// identical sentence -- Cave of the Frost Dragon, Den of the Bugbear, Hall
/// of Storm Giants, Hive of the Eye Tyrant -- all write `at_most: 1`, so it
/// came down *untapped* off exactly two other lands and every other card in
/// its own cycle came down tapped. Two other lands is the boundary the
/// wrong bound sat on, which is why this is the side that is played: it is
/// the assertion that fails against the code it replaces.
#[test]
fn a_manland_that_reads_two_or_more_other_lands_enters_tapped_off_exactly_two() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(960, forest())
        .battlefield(0, &[forest(), forest()])
        .hand(0, &[lair_of_the_hydra()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let lair = play_land(&mut engine, p0, lair_of_the_hydra());
    pass_until(&mut engine, stack_is_empty);
    assert!(
        entered_tapped(&engine, lair),
        "two other lands is \"two or more\", so the Lair enters tapped"
    );
}

/// The other side of the same bound, one land fewer.
///
/// A test that only asserted the tapped half would pass against a filter
/// that had simply been written "always tapped", so the untapped side is
/// what makes the number mean anything.
#[test]
fn a_manland_that_reads_two_or_more_other_lands_enters_untapped_off_one() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(961, forest())
        .battlefield(0, &[forest()])
        .hand(0, &[lair_of_the_hydra()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let lair = play_land(&mut engine, p0, lair_of_the_hydra());
    pass_until(&mut engine, stack_is_empty);
    assert!(
        !entered_tapped(&engine, lair),
        "one other land is under the bound, so the Lair enters untapped"
    );
}

/// Lair of the Hydra enters tapped if you control two or more other lands,
/// prints `{{T}}: Add {{G}}`, and `{{X}}{{G}}: Until end of turn, this land
/// becomes an X/X green Hydra creature. It's still a land. X can't be 0.`
///
/// This test was written as its own opposite and is kept as the thing it
/// became. Until CR 602.2b was implemented **X was never announced**: the
/// engine reached `Pending::ChooseNumber` only through `counter_x_part`,
/// which reads "remove X storage counters" and nothing about mana, so
/// `{{X}}{{G}}` was paid as `{{G}}` and the land became a 0/0 that a
/// state-based action put in the graveyard before anybody could attack with
/// it. The card could not do the thing it prints, whatever number its
/// controller had in mind. Three other cards carried the same silent zero —
/// Treasure Vault, Kessig Wolf Run and Blast Zone — so it was one rule and
/// four cards.
///
/// What is asserted now is the announcement itself and not merely the
/// outcome, because a 2/2 is also what a hard-coded two would produce. The
/// bound is the interesting half: `max` is what the **pool** can pay, which
/// is where an activation differs from a cast: the cast wizard starts
/// with a conservative resource bound and validates later choices at payment.
///
/// `min` is 0 and the card says X can't be 0, which is the whole of its
/// remaining `Coverage::Partial`: a lower bound printed on the card has no
/// field on a cost.
#[test]
fn a_mana_x_in_an_activation_cost_is_announced_and_bounded_by_the_pool() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[lair_of_the_hydra(), forest(), forest(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let lair = on_battlefield(&engine, p0, lair_of_the_hydra()).expect("lair on battlefield");
    assert!(
        !engine
            .state()
            .object(lair)
            .expect("lair exists")
            .characteristics()
            .types
            .contains(TypeSet::CREATURE),
        "a land is not a creature before activation"
    );

    tap_all_mana_but(&mut engine, p0, Some(lair_of_the_hydra()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        3,
        "three Forests, which is {{X}}{{G}} with X = 2"
    );
    activate(&mut engine, p0, lair_of_the_hydra(), 1);

    let Pending::ChooseNumber {
        player, min, max, ..
    } = engine.pending().clone()
    else {
        panic!(
            "CR 602.2b announces the number with the activation, and the \
             pending is {:?}",
            engine.pending()
        );
    };
    assert_eq!(player, p0);
    assert_eq!(
        (min, max),
        (0, 2),
        "three mana floating pays {{X}}{{G}} up to X = 2, and the lower \
         bound the card prints has nowhere to be written"
    );
    assert!(
        engine.apply(p0, PlayerAction::ChooseNumber(3)).is_err(),
        "a number the pool cannot pay is refused at the question rather \
         than at the payment"
    );
    engine
        .apply(p0, PlayerAction::ChooseNumber(2))
        .expect("announce X = 2");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the announced number *is* the cost by the time it is paid, so all \
         three were spent"
    );

    pass_until(&mut engine, stack_is_empty);
    let lair = on_battlefield(&engine, p0, lair_of_the_hydra())
        .expect("a 2/2 stays on the battlefield where a 0/0 did not");
    let chars = engine
        .state()
        .object(lair)
        .expect("lair exists")
        .characteristics();
    assert!(chars.types.contains(TypeSet::CREATURE));
    assert!(
        chars.types.contains(TypeSet::LAND),
        "\"It's still a land.\""
    );
    assert!(
        chars
            .subtypes
            .contains(baylee_core::generated::subtypes::creature::HYDRA)
    );
    assert_eq!(pt(&engine, lair), (2, 2), "an X/X with X announced as 2");
}
