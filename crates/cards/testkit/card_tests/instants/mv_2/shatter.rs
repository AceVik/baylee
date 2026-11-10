//! `cards/instants/mv_2/shatter.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Shatter — {1}{R} instant: "Destroy target artifact."
///
/// The target is the whole card, so the menu is what proves the printed word
/// `artifact` was read and not skipped: two Sol Rings — one under each seat —
/// are on it while the Llanowar Elves beside them and every land on the table
/// are not, because the filter names a type and no side of the battlefield.
/// Answering with the opponent's Ring and letting the spell resolve reads the
/// other half of the sentence — the card in its owner's graveyard, my own Ring
/// untouched, the declined creature still standing — and the battlefield read
/// while the question is open is CR 601.2c before CR 601.2h: the cost is the
/// last step of the cast, so nothing has moved yet when the target is named.
#[test]
fn shatter_destroys_the_artifact_it_names_and_no_other_permanent() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[mountain(), mountain(), quiet_artifact()])
        // An artifact and a creature across the table, so the filter has to
        // reach one and decline the other, and a land on my own side so
        // "artifact" is also read against a permanent I control.
        .battlefield(1, &[quiet_artifact(), llanowar_elves()])
        .hand(0, &[shatter()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let card = in_hand(&engine, p0, shatter()).expect("the spell is in hand");
    // `castable` is filtered through `can_afford`, which reads the pool and
    // not the untapped lands: with nothing floating the {1}{R} is unpayable,
    // so the claim is made only once the mana is really there.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        !legal.castable.contains(&card),
        "an empty pool pays no {{1}}{{R}}: {:?}",
        legal.castable
    );

    // `tap_all_mana` presses every mana ability whose whole price is its own
    // `{T}` (#159), and the Sol Ring beside the Mountains prints one — so it
    // is named as the thing kept back, which keeps it standing as the control
    // below and makes "two red" a count of the Mountains alone.
    tap_all_mana_but(&mut engine, p0, Some(quiet_artifact()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        2,
        "two Mountains tapped and the Sol Ring kept back: {{R}}{{R}}"
    );
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.castable.contains(&card),
        "with {{R}}{{R}} floating the spell is offered: {:?}",
        legal.castable
    );
    cast_with_floating(&mut engine, p0, shatter());

    let options = pass_until_targets(&mut engine, p0);
    let mine = on_battlefield(&engine, p0, quiet_artifact()).expect("my Sol Ring is out");
    let theirs = on_battlefield(&engine, p1, quiet_artifact()).expect("their Sol Ring is out");
    let elf = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elves are out");
    assert!(
        options.contains(&mine) && options.contains(&theirs),
        "\"target artifact\" reaches either side of the table: {options:?}"
    );
    assert!(
        !options.contains(&elf),
        "a creature is no artifact, whatever its controller: {options:?}"
    );
    for land in all_on_battlefield(&engine, p0, mountain()) {
        assert!(
            !options.contains(&land),
            "a land is no artifact either: {options:?}"
        );
    }
    assert!(
        on_battlefield(&engine, p1, quiet_artifact()).is_some(),
        "costs are paid last (CR 601.2h), so the Ring is still standing while \
         the target question is open"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![theirs],
            },
        )
        .expect("the Sol Ring the question offered is a legal target");

    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the {{1}}{{R}} came out of the pool as the spell was announced"
    );
    assert!(
        !stack_is_empty(&engine),
        "the spell is on the stack, waiting to resolve"
    );

    pass_until(&mut engine, stack_is_empty);

    assert!(
        in_graveyard(&engine, p1, quiet_artifact()).is_some(),
        "\"destroy target artifact\": the Ring is in its owner's graveyard"
    );
    assert!(
        on_battlefield(&engine, p1, quiet_artifact()).is_none(),
        "and has left the battlefield"
    );
    assert!(
        on_battlefield(&engine, p0, quiet_artifact()).is_some(),
        "the artifact the spell did not name never moved"
    );
    assert!(
        on_battlefield(&engine, p1, llanowar_elves()).is_some(),
        "and neither did the creature the filter declined"
    );
    assert!(
        in_graveyard(&engine, p0, shatter()).is_some(),
        "a resolved instant goes to its owner's graveyard"
    );
}
