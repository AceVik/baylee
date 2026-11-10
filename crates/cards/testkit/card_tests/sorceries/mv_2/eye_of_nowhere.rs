//! `cards/sorceries/mv_2/eye_of_nowhere.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Eye of Nowhere is `{U}{U}` for one line: "Return target permanent to its
/// owner's hand." "Permanent" is the whole card — a creature and a land, on
/// either side of the table, are all legal targets — and "its owner's" is the
/// word that decides which hand the card lands in once the spell is aimed
/// across the table, which a bounce of one's own creature could not tell from
/// "controller's". Two Islands pay the `{U}{U}` with the Elf kept back (its
/// own `{T}: Add {G}` is a mana route too), so the blue is real mana out of
/// the pool, and it is still floating while the target question stands
/// (CR 601.2c before CR 601.2h).
#[test]
fn eye_of_nowhere_returns_any_permanent_to_its_owners_hand() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(), island(), llanowar_elves()])
        .battlefield(1, &[llanowar_elves(), forest()])
        .hand(0, &[eye_of_nowhere()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let my_lands = all_on_battlefield(&engine, p0, island());
    assert_eq!(my_lands.len(), 2, "two Islands are the {{U}}{{U}}");
    let mine = on_battlefield(&engine, p0, llanowar_elves()).expect("my Elves are out");
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elves are out");
    let their_land = on_battlefield(&engine, p1, forest()).expect("their Forest is out");

    // The two Islands, and the Elf named as the printing kept back: it is one
    // of the permanents the spell may name, and a source tapped for its own
    // mana would make "two blue" a claim about a board nothing accounts for.
    tap_all_mana_but(&mut engine, p0, Some(llanowar_elves()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        2,
        "two Islands tapped, and no creature on this board paid in"
    );
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    let spell = in_hand(&engine, p0, eye_of_nowhere()).expect("the spell is in hand");
    assert!(
        legal.castable.contains(&spell),
        "two blue in the pool pay {{U}}{{U}}: {:?}",
        legal.castable
    );
    cast_with_floating(&mut engine, p0, eye_of_nowhere());

    // "Any permanent": every permanent in the game is on the menu, on both
    // sides of the table and of both types the board offers.
    let options = options_offered_including(&mut engine, theirs);
    for id in [my_lands[0], my_lands[1], mine, theirs, their_land] {
        assert!(
            options.contains(&id),
            "\"target permanent\" reaches {id:?}: {options:?}"
        );
    }
    assert_eq!(
        options.len(),
        5,
        "and those five are the whole battlefield: {options:?}"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        2,
        "CR 601.2c names the target before CR 601.2h pays, so the mana is \
         still floating while the question stands"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![theirs],
            },
        )
        .expect("the Elf across the table was one of the options");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the {{U}}{{U}} came out of the pool"
    );
    assert!(
        on_stack(&engine, eye_of_nowhere()).is_some(),
        "and the spell is on the stack, waiting to resolve"
    );

    pass_until(&mut engine, stack_is_empty);

    assert!(
        in_hand(&engine, p1, llanowar_elves()).is_some(),
        "\"to its owner's hand\": the Elf goes to the seat that owns it, not \
         to the seat that aimed the spell"
    );
    assert!(
        in_hand(&engine, p0, llanowar_elves()).is_none(),
        "the caster keeps nothing of what it bounced"
    );
    assert!(
        on_battlefield(&engine, p1, llanowar_elves()).is_none(),
        "the targeted permanent left the battlefield"
    );
    assert!(
        on_battlefield(&engine, p1, forest()).is_some(),
        "the permanent the spell did not name never moved"
    );
    assert_eq!(
        all_on_battlefield(&engine, p0, island()).len(),
        2,
        "and neither did the two Islands that paid for it"
    );
    assert!(
        on_battlefield(&engine, p0, llanowar_elves()).is_some(),
        "nor the creature that was kept back"
    );
    assert!(
        in_graveyard(&engine, p0, eye_of_nowhere()).is_some(),
        "a resolved sorcery goes to its owner's graveyard"
    );
}
