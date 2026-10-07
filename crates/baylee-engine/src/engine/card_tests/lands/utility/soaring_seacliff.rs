//! `cards/lands/utility/soaring_seacliff.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Soaring Seacliff is free and prints three lines, so a single play reads all
/// of them: it enters tapped, its enters-trigger grants flying until end of
/// turn, and its `{T}: Add {U}` does not exist at all until an untap step has
/// stood it back up. The target is the creature across the table, which is
/// what tells "target creature" from "target creature you control" — my own
/// Elf stands beside it and has to stay grounded — and the turn walked
/// afterwards is what tells the printed duration from a permanent grant.
#[test]
#[allow(clippy::too_many_lines)]
fn soaring_seacliff_enters_tapped_grants_flying_and_taps_for_blue_a_turn_later() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[llanowar_elves()])
        .battlefield(1, &[llanowar_elves()])
        .hand(0, &[soaring_seacliff()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let mine = on_battlefield(&engine, p0, llanowar_elves()).expect("my Elf is out");
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elf is out");
    assert!(
        !keywords(&engine, mine).contains(KeywordSet::FLYING)
            && !keywords(&engine, theirs).contains(KeywordSet::FLYING),
        "nothing has granted anything yet"
    );

    let land = play_land(&mut engine, p0, soaring_seacliff());
    assert!(
        entered_tapped(&engine, land),
        "\"This land enters tapped\" — and a seeded `starting_battlefield` \
         could not have said so, because a placement is no entry"
    );

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    let Pending::ChooseTargets {
        player,
        options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        unreachable!("the predicate just matched")
    };
    assert_eq!(player, p0, "the land's controller names the target");
    assert_eq!(
        (min, max),
        (1, 1),
        "one creature, and the trigger asks once"
    );
    assert!(
        options.contains(&mine) && options.contains(&theirs),
        "\"target creature\" is any creature, on either side of the table: {options:?}"
    );
    assert!(
        !options.contains(&land),
        "a land is no creature, so it cannot be hit by its own trigger: {options:?}"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![theirs],
            },
        )
        .expect("the Elf across the table was one of the options");
    pass_until(&mut engine, |e| at_rest(e, p0));

    assert!(
        keywords(&engine, theirs).contains(KeywordSet::FLYING),
        "\"target creature gains flying until end of turn\""
    );
    assert!(
        !keywords(&engine, mine).contains(KeywordSet::FLYING),
        "the trigger reaches the creature it named and no other: \"target \
         creature\" is not \"creatures you control\""
    );
    assert!(
        !keywords(&engine, land).contains(KeywordSet::FLYING),
        "the land grants the keyword, it does not keep it"
    );
    assert!(
        is_tapped(&engine, land),
        "and the entry that tapped it is the entry it is still lying in"
    );

    // "until end of turn": a turn later the Elf is grounded again, which a
    // static or an endlessly-dated grant would not show.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert!(
        !keywords(&engine, theirs).contains(KeywordSet::FLYING),
        "the grant lasted the turn it was made in and no longer"
    );
    assert!(
        on_battlefield(&engine, p1, llanowar_elves()).is_some(),
        "and the creature is still standing, so the keyword left rather than the creature"
    );

    // The third printed line needs the untap step, which has now run: `{T}` on
    // a land that entered tapped is not an offer in its arrival turn.
    assert!(
        !is_tapped(&engine, land),
        "the untap step stood the land back up (CR 502.3)"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "and the pool emptied with the step that ended (CR 500.5)"
    );
    let taken = tap_mana_where(&mut engine, p0, |id| id == land);
    assert_eq!(taken, 1, "the one route is the land's own {{T}}: Add {{U}}");
    assert!(is_tapped(&engine, land), "the land paid its own {{T}}");
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Blue),
        1,
        "{{U}} — the one color the card prints"
    );
    assert_eq!(
        pool.total(),
        1,
        "and nothing else: the Elves were never pressed, because the route \
         named the land and nothing but the land"
    );
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack, so nothing is waiting to resolve"
    );
}
