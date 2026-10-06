//! `cards/lands/utility/shinka_the_bloodsoaked_keep.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

// oracle_id = "255b937f-c9c9-4ae9-815e-17418eba0602"

/// Shinka, the Bloodsoaked Keep is a legendary land printing two abilities:
/// "{T}: Add {R}" and "{R}, {T}: Target legendary creature gains first strike
/// until end of turn." Both are played on one board, because neither is
/// something the card file can be read for: the red the land makes is the only
/// red left once the two Mountains beside it are tapped, and the `{R}, {T}` line
/// is a real payment — the land is still untapped and the pool still full while
/// the target question stands (CR 601.2c before CR 601.2h). The menu that
/// question publishes is the card's filter, so a legendary creature under this
/// seat and one across the table are both on it while a non-legendary creature
/// is not, which is what tells "target legendary creature" from "target
/// legendary creature you control". The printed "until end of turn" is walked
/// out rather than read.
#[test]
#[allow(clippy::too_many_lines)] // one printed card, played end to end: the length is the card's
fn shinka_taps_for_red_and_charges_red_and_its_tap_for_a_legendary_creature_to_strike_first() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                mountain(),
                mountain(),
                muldrotha_the_gravetide(),
                serra_angel(),
            ],
        )
        .battlefield(1, &[aesi_tyrant_of_gyre_strait()])
        .hand(0, &[shinka_the_bloodsoaked_keep()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // Played and not seated: a land drop is how the card arrives, and the
    // permanent it becomes is the one both printed lines are read off.
    let keep = play_land(&mut engine, p0, shinka_the_bloodsoaked_keep());
    assert!(
        !is_tapped(&engine, keep),
        "a land that prints no enter modifier enters untapped, so its {{T}} is payable"
    );

    let troll =
        on_battlefield(&engine, p0, muldrotha_the_gravetide()).expect("my legendary creature");
    let angel =
        on_battlefield(&engine, p0, serra_angel()).expect("my creature, which is no legend");
    let theirs = on_battlefield(&engine, p1, aesi_tyrant_of_gyre_strait())
        .expect("a legendary creature across the table");
    let land = on_battlefield(&engine, p0, mountain()).expect("a Mountain is out");
    assert!(
        !keywords(&engine, troll).contains(KeywordSet::FIRST_STRIKE),
        "nothing has granted anything yet"
    );

    // Mana before the claim: `can_afford` reads the pool and not the untapped
    // lands, so the `{R}, {T}` line is only in the offer once the red is
    // floating. Shinka is named as the printing kept back — its own printed
    // `{T}: Add {R}` has the tap for its whole price, so `tap_all_mana` would
    // have spent the very `{T}` the activated ability charges (#159).
    tap_all_mana_but(&mut engine, p0, Some(shinka_the_bloodsoaked_keep()));
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Red),
        2,
        "two Mountains, and neither the legendary creature nor the Angel makes mana"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        2,
        "so two red is the whole pool"
    );

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(keep, 0)),
        "\"{{T}}: Add {{R}}\" is the first ability the card prints, and the land \
         is untapped: {:?}",
        legal.abilities
    );
    assert!(
        legal.abilities.contains(&(keep, 1)),
        "and \"{{R}}, {{T}}: Target legendary creature gains first strike …\" is \
         the second, now that its {{R}} is floating: {:?}",
        legal.abilities
    );
    assert!(
        !is_tapped(&engine, keep),
        "the source was kept back, so its {{T}} is still there to pay with"
    );

    activate(&mut engine, p0, shinka_the_bloodsoaked_keep(), 1);
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
    assert_eq!(player, p0, "the activating seat is the one that aims it");
    assert_eq!(
        (min, max),
        (1, 1),
        "one creature, and the ability asks once"
    );
    assert!(
        options.contains(&troll),
        "the legendary creature under this seat is on the menu: {options:?}"
    );
    assert!(
        options.contains(&theirs),
        "and so is the legendary creature across the table — the card prints \
         \"target legendary creature\" and no \"you control\": {options:?}"
    );
    assert!(
        !options.contains(&angel),
        "Serra Angel is a creature and not a legendary one: {options:?}"
    );
    assert!(
        !options.contains(&keep),
        "Shinka is a land and no creature, so it is no target for its own \
         ability: {options:?}"
    );
    assert_eq!(
        options.len(),
        2,
        "and those two are the whole menu: {options:?}"
    );
    // CR 601.2c names the target first and CR 601.2h pays afterwards: while the
    // question stands, the land is still untapped and the red still in the pool.
    assert!(
        !is_tapped(&engine, keep),
        "the {{T}} is the last step of the activation, not the first"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        2,
        "and the {{R}} is still floating for the same reason"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![troll],
            },
        )
        .expect("the legendary creature the question offered is a legal answer");

    assert!(
        is_tapped(&engine, keep),
        "{{T}} is half the price and is paid with the rest"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        1,
        "and the {{R}} came out of the pool"
    );
    assert!(
        !stack_is_empty(&engine),
        "granting a keyword is no mana ability, so the ability is on the stack"
    );

    pass_until(&mut engine, stack_is_empty);

    assert!(
        keywords(&engine, troll).contains(KeywordSet::FIRST_STRIKE),
        "the legendary creature the ability named gained first strike"
    );
    assert!(
        !keywords(&engine, angel).contains(KeywordSet::FIRST_STRIKE),
        "the non-legendary creature nobody named is untouched: the effect \
         targets, it does not sweep the board"
    );
    assert!(
        !keywords(&engine, theirs).contains(KeywordSet::FIRST_STRIKE),
        "and it never reaches the legendary creature that was offered but not \
         aimed at"
    );
    assert!(
        !keywords(&engine, keep).contains(KeywordSet::FIRST_STRIKE),
        "the land grants the keyword, it does not keep it"
    );

    // "until end of turn", played rather than read: a whole turn later the
    // creature is still standing and the keyword is not.
    reach_their_main_phase(&mut engine, p1);
    assert!(
        walk_to_own_main(&mut engine, p0),
        "Shinka's controller takes another turn"
    );
    assert!(
        !keywords(&engine, troll).contains(KeywordSet::FIRST_STRIKE),
        "the grant lasted the turn it was made in and no longer"
    );
    assert!(
        !is_tapped(&engine, keep),
        "the untap step stood Shinka back up, which is what makes its {{T}} \
         payable again"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "and the pool emptied with the step that ended (CR 500.5)"
    );

    // The first printed line, on the turn its {{T}} came back. Nothing has been
    // tapped this turn, so the one red in the pool is the land's own.
    activate(&mut engine, p0, shinka_the_bloodsoaked_keep(), 0);
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack, so the mana is already here"
    );
    assert!(is_tapped(&engine, keep), "the land paid its own {{T}}");
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Red), 1, "\"{{T}}: Add {{R}}\"");
    assert_eq!(
        pool.total(),
        1,
        "one mana, off one tap, and nothing beside it"
    );
    assert!(
        !is_tapped(&engine, land),
        "the Mountain beside it is still standing, so the red in the pool has no \
         other source on this board"
    );
}
