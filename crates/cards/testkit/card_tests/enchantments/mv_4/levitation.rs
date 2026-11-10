//! `cards/enchantments/mv_4/levitation.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

// oracle_id = "94b703c4-5584-4913-8365-7e9f2f535c2d"

/// Levitation is `{2}{U}{U}` for one printed sentence: "Creatures you control
/// have flying." So the reading worth playing is the one that tells which
/// creatures the static *names* — two Elves of mine against one Elf across the
/// table, with the keyword read off the layer projection rather than off the
/// card file. Both halves of "you control" need a witness, and the same card is
/// the only thing that can supply them: a filter that had widened to
/// `Filter::CREATURE` would arm the Elf opposite, and one that had lost
/// `Filter::CREATURE` altogether would arm the enchantment itself, which the
/// last assertion reads at zero. The board before the cast is the control for
/// the board after it, since a creature that was already flying would satisfy
/// every positive claim for a reason that has nothing to do with Levitation
/// arriving.
#[test]
fn levitation_grants_flying_to_the_creatures_you_control_and_no_other() {
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
                llanowar_elves(),
                llanowar_elves(),
            ],
        )
        .hand(0, &[levitation()])
        .battlefield(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let mine = all_on_battlefield(&engine, p0, llanowar_elves());
    assert_eq!(
        mine.len(),
        2,
        "two Elves of mine, one of which is the control"
    );
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elf is out");
    for id in &mine {
        assert!(
            !keywords(&engine, *id).contains(KeywordSet::FLYING),
            "with no Levitation on the battlefield a printed 1/1 is grounded"
        );
    }
    assert!(
        !keywords(&engine, theirs).contains(KeywordSet::FLYING),
        "and so is the Elf across the table"
    );

    // Four Islands are exactly `{2}{U}{U}`, and both Elves are named as the
    // printing kept back: they are the creatures this test reads afterwards,
    // and `tap_all_mana` would have spent their own `{T}: Add {G}` as well
    // (#159) — the offer is read off the pool, so the mana has to be really
    // there before the cast is claimed.
    tap_all_mana_but(&mut engine, p0, Some(llanowar_elves()));
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Blue),
        4,
        "four Islands tapped for four blue and nothing else contributed"
    );
    cast_with_floating(&mut engine, p0, levitation());
    pass_until(&mut engine, stack_is_empty);

    let enchantment = on_battlefield(&engine, p0, levitation()).expect("Levitation resolved");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the {{2}}{{U}}{{U}} came out of the pool: an enchantment that had \
         resolved for free would land the keyword just as well"
    );
    let kinds = types(&engine, enchantment);
    assert!(
        kinds.contains(TypeSet::ENCHANTMENT) && !kinds.contains(TypeSet::CREATURE),
        "the permanent that arrived is the enchantment it prints: {kinds:?}"
    );

    for id in &mine {
        assert!(
            keywords(&engine, *id).contains(KeywordSet::FLYING),
            "\"Creatures you control have flying\" — every creature of mine, \
             not only the first one the filter looked at"
        );
    }
    assert!(
        !keywords(&engine, theirs).contains(KeywordSet::FLYING),
        "\"you control\" is not \"the table\": the Elf across it is a creature \
         and not mine"
    );
    assert!(
        !keywords(&engine, enchantment).contains(KeywordSet::FLYING),
        "the enchantment grants the keyword, it does not keep it — a static \
         that had reached its own source would show flying on an enchantment"
    );
}
