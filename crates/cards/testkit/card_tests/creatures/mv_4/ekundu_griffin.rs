//! `cards/creatures/mv_4/ekundu_griffin.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Ekundu Griffin prints a mana cost and two keywords and nothing else:
/// "{3}{W}" for a 2/2 Griffin with flying and first strike. A keyword is
/// exactly the characteristic that keeps reading correctly while the rest of a
/// card is missing, so the scenario pays for the card for real: four Plains
/// fill the pool, `{3}{W}` empties it, and the Griffin has to arrive as the
/// printed body with both keywords projected onto it. An Elf under the same
/// seat and another across the table are the control — neither may pick up
/// flying or first strike, which is what tells a keyword this card prints from
/// a grant that swept the board.
#[test]
fn ekundu_griffin_lands_as_a_two_two_with_flying_and_first_strike() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, plains())
        .battlefield(
            0,
            &[plains(), plains(), plains(), plains(), llanowar_elves()],
        )
        .hand(0, &[ekundu_griffin()])
        .battlefield(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let bystander = on_battlefield(&engine, p0, llanowar_elves()).expect("my Elf is out");
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elf is out");
    assert!(
        !keywords(&engine, bystander).contains(KeywordSet::FLYING),
        "nothing on this board has been granted flying yet"
    );

    // Four Plains, and the Elf named as the printing kept back: it is the
    // control this test reads afterwards, and `tap_all_mana` would have spent
    // its own `{T}: Add {G}` as well (#159).
    tap_all_mana_but(&mut engine, p0, Some(llanowar_elves()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        4,
        "four tapped Plains and four white, with the Elf contributing nothing"
    );

    cast_with_floating(&mut engine, p0, ekundu_griffin());
    pass_until(&mut engine, stack_is_empty);
    let griffin = on_battlefield(&engine, p0, ekundu_griffin()).expect("the Griffin resolved");

    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the {{3}}{{W}} came out of the pool the Plains filled"
    );
    assert!(
        types(&engine, griffin).contains(TypeSet::CREATURE),
        "the card resolved onto the battlefield as a creature"
    );
    assert_eq!(pt(&engine, griffin), (2, 2), "the body the card prints");

    let printed = keywords(&engine, griffin);
    assert!(
        printed.contains(KeywordSet::FLYING),
        "Ekundu Griffin has flying"
    );
    assert!(
        printed.contains(KeywordSet::FIRST_STRIKE),
        "and first strike"
    );
    assert!(
        !keywords(&engine, bystander).contains(KeywordSet::FLYING)
            && !keywords(&engine, bystander).contains(KeywordSet::FIRST_STRIKE),
        "the Elf nobody cast is still a printed 1/1 with no keywords"
    );
    assert!(
        !keywords(&engine, theirs).contains(KeywordSet::FLYING)
            && !keywords(&engine, theirs).contains(KeywordSet::FIRST_STRIKE),
        "and neither keyword reaches across the table"
    );
}
