//! `cards/creatures/mv_4/akroma_s_devoted.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Akroma's Devoted is a {3}{W} 2/4 Human Cleric whose whole text is "Cleric
/// creatures have vigilance." The filter carries no controller, so the test
/// stands a Cleric on each side of the table and a non-Cleric beside the
/// Devoted: only the two Clerics — the Devoted itself included, since it is
/// one — may pick the keyword up, and the Elf must not. That is what tells
/// the printed `Filter::HasSubtype(Cleric)` apart from a bare board-wide buff
/// or a "creatures you control" one, and the projected characteristics are the
/// only place that answer exists.
#[test]
fn akromas_devoted_grants_vigilance_to_the_clerics_of_both_seats_and_no_other_creature() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                plains(),
                plains(),
                plains(),
                plains(),
                ondu_cleric(),
                llanowar_elves(),
            ],
        )
        // A Cleric across the table: "Cleric creatures" names no controller.
        .battlefield(1, &[ondu_cleric()])
        .hand(0, &[akroma_s_devoted()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let cleric = on_battlefield(&engine, p0, ondu_cleric()).expect("my Cleric is out");
    let elves = on_battlefield(&engine, p0, llanowar_elves()).expect("my Elves are out");
    let theirs = on_battlefield(&engine, p1, ondu_cleric()).expect("their Cleric is out");
    assert!(
        !keywords(&engine, cleric).contains(KeywordSet::VIGILANCE),
        "nothing grants vigilance before the Devoted arrives"
    );

    // Four Plains pay the {3}{W}; the Elves are named as the printing kept
    // back so that the pool is the four lands and no creature's own mana.
    tap_all_mana_but(&mut engine, p0, Some(llanowar_elves()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        4,
        "four Plains tapped, and the Elves kept back"
    );
    cast_with_floating(&mut engine, p0, akroma_s_devoted());
    pass_until(&mut engine, stack_is_empty);

    let devoted = on_battlefield(&engine, p0, akroma_s_devoted()).expect("the Devoted resolved");
    assert_eq!(pt(&engine, devoted), (2, 4), "the body the card prints");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the {{3}}{{W}} came out of the pool"
    );

    assert!(
        keywords(&engine, devoted).contains(KeywordSet::VIGILANCE),
        "the Devoted is a Human Cleric itself, so its own static reaches it"
    );
    assert!(
        keywords(&engine, cleric).contains(KeywordSet::VIGILANCE),
        "and a Cleric under its controller"
    );
    assert!(
        keywords(&engine, theirs).contains(KeywordSet::VIGILANCE),
        "\"Cleric creatures\" names no controller, so the Cleric across the \
         table reads it too"
    );
    assert!(
        !keywords(&engine, elves).contains(KeywordSet::VIGILANCE),
        "an Elf Druid is no Cleric: the subtype filter is read and not skipped"
    );
}
