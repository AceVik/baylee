//! `cards/sorceries/mv_1/cloak_of_feathers.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Cloak of Feathers is `{U}` for "Target creature gains flying until end of
/// turn" and "Draw a card", and the two halves are read off two different
/// places: the keywords the layers hand the creature that was named, and the
/// library and hand the draw moved. The Elf across the table is the control
/// for "target creature" — it is offered and it must end the turn exactly as
/// it started — while p0's own Elf is named as the source kept back, so the
/// creature the spell lands on was not tapped for its own `{T}` first. The
/// spell is also read on the stack between its target and its resolution,
/// which is CR 601.2c before CR 601.2h.
#[test]
fn cloak_of_feathers_lifts_the_creature_it_names_and_draws_a_card() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(), llanowar_elves()])
        .hand(0, &[cloak_of_feathers()])
        .battlefield(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    assert!(
        walk_to_own_main(&mut engine, p0),
        "a sorcery wants p0's own main phase with an empty stack"
    );

    let mine = on_battlefield(&engine, p0, llanowar_elves()).expect("my Elf is out");
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elf is out");
    assert!(
        !keywords(&engine, mine).contains(KeywordSet::FLYING),
        "a printed 1/1 on the ground before the spell"
    );

    // The Elf is named as the source kept back: it prints its own
    // "{T}: Add {G}", so `tap_all_mana` would have spent the very creature
    // this spell is about to aim at.
    tap_mana_except(&mut engine, p0, mine);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        1,
        "the Island's blue, and nothing off the Elf"
    );

    let library_before = library_size(&engine, p0);
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();

    cast_with_floating(&mut engine, p0, cloak_of_feathers());
    let Pending::ChooseTargets {
        player, options, ..
    } = engine.pending().clone()
    else {
        panic!(
            "\"target creature\" is a target choice, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the casting seat is the one that aims it");
    assert!(
        options.contains(&mine) && options.contains(&theirs),
        "\"target creature\" reaches either side of the table: {options:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![mine],
            },
        )
        .expect("the creature the question offered is the one it lifts");

    // CR 601.2c named the target and CR 601.2h paid afterwards, so the spell
    // now stands on the stack and has reached neither of its later zones.
    assert!(
        on_stack(&engine, cloak_of_feathers()).is_some(),
        "the {{U}} is spent and the spell is waiting to resolve"
    );
    assert!(
        in_graveyard(&engine, p0, cloak_of_feathers()).is_none(),
        "a sorcery reaches the graveyard when it resolves, not when it is cast"
    );

    pass_until(&mut engine, stack_is_empty);

    assert!(
        keywords(&engine, mine).contains(KeywordSet::FLYING),
        "\"target creature gains flying until end of turn\""
    );
    assert_eq!(
        pt(&engine, mine),
        (1, 1),
        "and nothing but the keyword: the pump the card carries prints 0/0"
    );
    assert!(
        !keywords(&engine, theirs).contains(KeywordSet::FLYING),
        "the spell lifts the creature it named and never across the table"
    );
    assert_eq!(
        library_size(&engine, p0),
        library_before - 1,
        "\"Draw a card\": one card left the top of the library"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before,
        "the spell left the hand and the draw filled the seat back up, so a \
         cast that never drew would read one short"
    );
    assert!(
        in_graveyard(&engine, p0, cloak_of_feathers()).is_some(),
        "and the sorcery is in its owner's graveyard"
    );

    // The duration is "until end of turn" and not "for the rest of the game":
    // the opponent's main phase lies past p0's cleanup, where it ends.
    reach_their_main_phase(&mut engine, p1);
    assert!(
        !keywords(&engine, mine).contains(KeywordSet::FLYING),
        "the ground is where the Elf started the turn and where it ends it"
    );
}
