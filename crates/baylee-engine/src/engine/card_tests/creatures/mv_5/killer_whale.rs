//! `cards/creatures/mv_5/killer_whale.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Killer Whale is a `{3}{U}{U}` 3/5 Whale whose entire printed text is
/// "`{U}`: This creature gains flying until end of turn."
///
/// One board answers every word of that sentence. Six Islands pay the cast and
/// leave exactly the one blue the ability charges, so the mana the pool loses is
/// the printed price and not a land that happened to move; the Elf beside the
/// Whale and the Elf across the table are what tells "this creature" from
/// "creatures you control"; and a whole turn round the table leaves the Whale
/// standing with the keyword gone, which a static grant could not do. The empty
/// pool afterwards carries the other half of the price — the same ability on the
/// same board is no longer offered, because `can_afford` reads the pool and not
/// the untapped lands.
#[test]
fn killer_whale_pays_a_blue_for_flying_until_the_turn_ends_and_for_nobody_else() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, island())
        .battlefield(
            0,
            &[
                island(),
                island(),
                island(),
                island(),
                island(),
                island(),
                llanowar_elves(),
            ],
        )
        .hand(0, &[killer_whale()])
        .battlefield(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let elf = on_battlefield(&engine, p0, llanowar_elves()).expect("the Elf is out");
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elf is out");

    // Six Islands into the pool and the Elf kept back: it prints a `{T}: Add
    // {G}` of its own, and it is one of the creatures read back below.
    assert_eq!(
        tap_mana_except(&mut engine, p0, elf),
        6,
        "six Islands, and the Elf contributed nothing"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        6,
        "six blue, which is the {{3}}{{U}}{{U}} and the ability's {{U}} together"
    );
    cast_with_floating(&mut engine, p0, killer_whale());
    pass_until(&mut engine, |e| at_rest(e, p0));

    let whale = on_battlefield(&engine, p0, killer_whale()).expect("the Whale resolved");
    assert_eq!(pt(&engine, whale), (3, 5), "the body the card prints");
    assert!(
        !keywords(&engine, whale).contains(KeywordSet::FLYING),
        "and no flying until the ability is paid for"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        1,
        "six Islands less the {{3}}{{U}}{{U}}: exactly the {{U}} the ability charges"
    );

    // The offer is read off the *pool*, so the claim is made with the blue
    // already floating.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(whale, 0)),
        "the one line the card prints is offered now that its {{U}} is payable: {:?}",
        legal.abilities
    );

    activate(&mut engine, p0, killer_whale(), 0);
    assert!(
        !stack_is_empty(&engine),
        "gaining a keyword is no mana ability, so the ability is on the stack"
    );
    pass_until(&mut engine, |e| at_rest(e, p0));

    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the {{U}} came out of the pool"
    );
    assert!(
        keywords(&engine, whale).contains(KeywordSet::FLYING),
        "\"{{U}}: This creature gains flying until end of turn\""
    );
    assert!(
        !keywords(&engine, elf).contains(KeywordSet::FLYING),
        "\"this creature\" is not \"creatures you control\": the Elf beside it is untouched"
    );
    assert!(
        !keywords(&engine, theirs).contains(KeywordSet::FLYING),
        "nor across the table"
    );

    // The other half of the price, on the same board with only the pool
    // changed: `can_afford` reads the mana, so an empty one offers nothing.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        !legal.abilities.contains(&(whale, 0)),
        "the pool is empty, so the {{U}} is unpayable and the line is not offered: {:?}",
        legal.abilities
    );

    // "until end of turn": a full turn round the table leaves the Whale
    // standing and the keyword gone, which a permanent grant could not do.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert!(
        on_battlefield(&engine, p0, killer_whale()).is_some(),
        "the Whale is still on the battlefield, so the keyword left rather than the creature"
    );
    assert!(
        !keywords(&engine, whale).contains(KeywordSet::FLYING),
        "the grant lasted the turn it was made in and no longer"
    );
}
