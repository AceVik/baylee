//! `cards/sorceries/mv_3/rain_of_tears.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Rain of Tears — {1}{B}{B} sorcery: "Destroy target land."
///
/// The two words the card turns on are "target" and "land", so the board
/// carries a witness for each: the Forest across the table is offered while
/// the Llanowar Elves standing beside it is not, which is the difference
/// between the printed filter and a widened `Filter::Any` or a creature
/// filter — every reading of the card file looks identical. The price is read
/// twice off the pool three Swamps actually filled, because `castable` and
/// `abilities` are filtered through `can_afford` and that reads the pool
/// rather than the untapped lands; and the destroyed land is followed into
/// its *owner's* graveyard, because "destroy" (CR 701.8) is neither an exile
/// nor a card that changes hands.
#[test]
#[allow(clippy::too_many_lines)] // one printed spell, played end to end: the length is the card's
fn rain_of_tears_destroys_the_land_it_names_and_never_the_creature_beside_it() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[swamp(), swamp(), swamp()])
        .battlefield(1, &[forest(), llanowar_elves()])
        .hand(0, &[rain_of_tears()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let mine = on_battlefield(&engine, p0, swamp()).expect("my Swamp is out");
    let theirs = on_battlefield(&engine, p1, forest()).expect("their Forest is out");
    let elf = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elf is out");

    // An empty pool pays no {1}{B}{B}, and `can_afford` reads the pool rather
    // than the three untapped Swamps — so the sorcery is not castable until
    // the mana is really floating, which is the claim the offer is read on.
    let card = in_hand(&engine, p0, rain_of_tears()).expect("the sorcery is in hand");
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        !legal.castable.contains(&card),
        "an empty pool pays no {{1}}{{B}}{{B}}: {:?}",
        legal.castable
    );

    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Black),
        3,
        "three Swamps tapped, three black"
    );

    cast_with_floating(&mut engine, p0, rain_of_tears());
    let Pending::ChooseTargets {
        player,
        options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        panic!(
            "\"target land\" is a target choice, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the seat that cast it names the target");
    assert_eq!((min, max), (1, 1), "one land, and the spell asks once");
    assert!(
        options.contains(&mine) && options.contains(&theirs),
        "\"target land\" is any land, on either side of the table: {options:?}"
    );
    assert!(
        !options.contains(&elf),
        "\"target land\" is read, not skipped: a creature is no legal target \
         for it, however close it stands to the Forest: {options:?}"
    );
    assert!(
        on_battlefield(&engine, p1, forest()).is_some(),
        "CR 601.2c before CR 601.2h: the target is named while the land it \
         names is still on the battlefield"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![theirs],
            },
        )
        .expect("the land was one of the options the question enumerated");

    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the {{1}}{{B}}{{B}} came out of the pool"
    );
    assert!(
        !stack_is_empty(&engine),
        "destroying a land is no mana ability, so the spell is on the stack"
    );
    assert!(
        on_battlefield(&engine, p1, forest()).is_some(),
        "and the destruction is the resolution, not the cost"
    );

    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, p1, forest()).is_none(),
        "\"destroy target land\" took the land the spell named off the battlefield"
    );
    assert!(
        in_graveyard(&engine, p1, forest()).is_some(),
        "and a destroyed permanent goes to its owner's graveyard — under the \
         seat that owns it, not under the seat that cast the spell"
    );
    assert!(
        in_graveyard(&engine, p0, rain_of_tears()).is_some(),
        "the sorcery itself resolved and is in the graveyard of the seat that cast it"
    );
    assert!(
        on_battlefield(&engine, p0, swamp()).is_some(),
        "the land the spell did not name never moved, and mine is still standing"
    );
    assert!(
        on_battlefield(&engine, p1, llanowar_elves()).is_some(),
        "the creature beside the Forest was never on the menu, so it never moved"
    );
    assert_eq!(
        (
            engine.state().players[0].life,
            engine.state().players[1].life
        ),
        (20, 20),
        "the card destroys a land and nothing else: no life total moved"
    );
}
