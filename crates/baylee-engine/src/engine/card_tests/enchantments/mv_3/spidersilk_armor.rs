//! `cards/enchantments/mv_3/spidersilk_armor.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Spidersilk Armor — {2}{G} — "Creatures you control get +0/+1 and have
/// reach."
///
/// Two printed statics, and every word in them gets a witness on this board.
/// The pump is read as `(1, 2)` on a printed 1/1 — a `(2, 2)` would mean
/// `+1/+1` was read instead — while the Serra Angel across the table keeps the
/// body it was seated with, which is what tells "creatures you control" from
/// "creatures". Reach is a blocking permission and nothing besides, so it is
/// not read off the keyword set alone: the same Elf is left standing in front
/// of a flying attacker and the engine has to offer it as a blocker, which is
/// the only place that keyword does anything at all.
#[test]
fn spidersilk_armor_pumps_and_arms_your_creatures_with_reach_and_no_one_elses() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), forest(), llanowar_elves()])
        .battlefield(1, &[serra_angel()])
        .hand(0, &[spidersilk_armor()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let elf = on_battlefield(&engine, p0, llanowar_elves()).expect("the Elves are out");
    let angel = on_battlefield(&engine, p1, serra_angel()).expect("the Angel is out");
    let angel_body = pt(&engine, angel);
    assert_eq!(pt(&engine, elf), (1, 1), "a printed 1/1 before the Armor");
    assert!(
        !keywords(&engine, elf).contains(KeywordSet::REACH),
        "a printed Llanowar Elves has no reach of its own"
    );

    // The Elf is named as the printing kept back: it prints its own
    // `{T}: Add {G}`, so `tap_all_mana` would have drunk it (#159) — and a
    // creature tapped for mana is a creature that cannot block, which is the
    // half of this scenario the reach reading below depends on.
    tap_all_mana_but(&mut engine, p0, Some(llanowar_elves()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        3,
        "three Forests, exactly the {{2}}{{G}} the Armor prints"
    );
    cast_with_floating(&mut engine, p0, spidersilk_armor());
    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, p0, spidersilk_armor()).is_some(),
        "the Armor resolved onto the table"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "and the three green went into its {{2}}{{G}}"
    );
    assert_eq!(
        pt(&engine, elf),
        (1, 2),
        "\"get +0/+1\": the power the Elf was printed with and one more \
         toughness — a (2, 2) would mean the wrong half of the pump was read"
    );
    assert!(
        keywords(&engine, elf).contains(KeywordSet::REACH),
        "\"creatures you control … have reach\" reaches the Elf"
    );
    assert_eq!(
        pt(&engine, angel),
        angel_body,
        "an opponent's creature is not a creature you control: the Angel is \
         still the body it was seated as"
    );
    assert!(
        !keywords(&engine, angel).contains(KeywordSet::REACH),
        "and the static is scoped to one side of the table, not to the board"
    );

    // Reach only does anything against a flier, so the proof is the block the
    // engine offers: the printed 1/1 that could not block the Angel before the
    // Armor is on that list now, and the Angel is the only attacker there is.
    reach_their_main_phase(&mut engine, p1);
    let blocks = attack_and_collect_blocks(&mut engine, angel, p0);
    assert!(
        blocks
            .iter()
            .any(|option| option.blocker == elf && option.attackers.contains(&angel)),
        "\"creatures you control … have reach\": the Elf is offered as a \
         blocker of the flying Angel: {blocks:?}"
    );
}
