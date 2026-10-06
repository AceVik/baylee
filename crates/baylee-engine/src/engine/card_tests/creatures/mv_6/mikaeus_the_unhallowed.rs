//! `cards/creatures/mv_6/mikaeus_the_unhallowed.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Mikaeus, the Unhallowed prints one sentence this card can say — "other
/// non-Human creatures you control get +1/+1" — and two it cannot. One board
/// strikes all three words of the one at once: a Llanowar Elves beside him is
/// a 2/2 while he stays the printed 5/5 ("other", so he pumps no part of
/// himself) and the Elves across the table stays the printed 1/1 ("you
/// control").
///
/// The rest is the `Coverage::Partial` gap, and it is struck rather than left
/// implied, because the first draft of this card claimed both keywords and
/// nothing at the table changed: intimidate is a bit no engine rule reads, and
/// the undying look-back reads only a printed undying, never a granted one. So
/// the same creature is fed to an Ashnod's Altar, and what the board says
/// afterwards is a dead Elf — which is what a granted keyword nobody reads
/// actually looks like.
#[test]
fn mikaeus_lords_the_nonhumans_beside_him_and_the_undying_half_is_not_granted() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(88, swamp())
        .battlefield(
            0,
            &[mikaeus_the_unhallowed(), ashnods_altar(), llanowar_elves()],
        )
        .battlefield(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    // `walk_to_own_main` rather than `reach_main_phase`: which seat the seed
    // put on the play decides whether a whole turn is in the way.
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let mikaeus = on_battlefield(&engine, p0, mikaeus_the_unhallowed()).expect("Mikaeus stands");
    let elves = on_battlefield(&engine, p0, llanowar_elves()).expect("the Elves are out");
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elves are out");

    // The filter's third word, "non-Human", goes unstruck: no Human card is
    // named on this board.
    assert_eq!(
        pt(&engine, elves),
        (2, 2),
        "\"other non-Human creatures you control get +1/+1\""
    );
    assert_eq!(
        pt(&engine, mikaeus),
        (5, 5),
        "\"other\" — Mikaeus is a Zombie Cleric and no part of him is included"
    );
    assert_eq!(
        pt(&engine, theirs),
        (1, 1),
        "\"you control\" — the Elf across the table is the printed 1/1"
    );
    assert!(
        !keywords(&engine, elves).contains(KeywordSet::UNDYING),
        "the undying half of that same sentence is *not* granted: the \
         look-back never reads a granted undying, and a keyword nothing reads \
         would read as finished"
    );
    assert!(
        !keywords(&engine, mikaeus).contains(KeywordSet::INTIMIDATE),
        "nor is the keyword he prints for himself, for the same reason"
    );

    // The Altar costs nothing but the creature, so nothing but undying stands
    // between "dies" and "comes back".
    activate(&mut engine, p0, ashnods_altar(), 0);
    let Pending::ChooseCards {
        options, prompt, ..
    } = engine.pending().clone()
    else {
        panic!(
            "the Altar asks which creature to eat, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(
        prompt,
        crate::choice::ChoicePrompt::CostSacrifice,
        "a cost and not a search"
    );
    assert!(
        options.contains(&elves) && options.contains(&mikaeus),
        "both of this seat's creatures are food: {options:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![elves],
            },
        )
        .expect("the Elves pay the cost");

    // The trigger has to come and go before the board can be read.
    pass_until(&mut engine, stack_is_empty);

    // The gap, struck rather than left implied. The undying look-back asks
    // the card that died, not the keywords it had on the battlefield, so a
    // granted undying is gone by the time it is asked, and the card does not
    // grant it (the lint beside `keyword_tests::ENFORCED` is what caught this
    // card claiming intimidate). What that looks like at the table is exactly
    // this: the creature dies and stays dead.
    assert!(
        in_graveyard(&engine, p0, llanowar_elves()).is_some(),
        "no undying was granted, so the Elves lie where the Altar put them"
    );
    assert!(
        on_battlefield(&engine, p0, llanowar_elves()).is_none(),
        "and nothing returned them to the battlefield"
    );
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Colorless),
        2,
        "the Altar's own line did run: {{C}}{{C}} for the creature it ate"
    );
}
