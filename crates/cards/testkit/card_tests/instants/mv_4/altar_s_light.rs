//! `cards/instants/mv_4/altar_s_light.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

// oracle_id = "fa9b6be2-b88c-4302-b7e2-faf25a60bcb9"

/// Altar's Light — {2}{W}{W} instant: "Exile target artifact or
/// enchantment."
///
/// The whole card is one sentence with two nouns in it, so both halves are
/// played off one board that carries a witness for each word: an artifact and
/// an enchantment across the table are both offered, a creature is not, and
/// the two casts leave the artifact and then the enchantment sitting in their
/// owner's exile while the Elf never moves. Exile rather than graveyard is the
/// reading that tells the printed verb from a destroy, and the empty pool
/// before the first cast is what makes the missing `castable` a claim about
/// the {2}{W}{W} instead of about an absent target.
#[test]
fn altars_light_exiles_an_artifact_and_an_enchantment_and_declines_a_creature() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[plains(); 8])
        .hand(0, &[altars_light(), altars_light()])
        .battlefield(
            1,
            &[quiet_artifact(), their_enchantment(), llanowar_elves()],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let ring = on_battlefield(&engine, p1, quiet_artifact()).expect("their Sol Ring is out");
    let chant = on_battlefield(&engine, p1, their_enchantment()).expect("their enchantment is out");
    let elf = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elf is out");

    // Nothing floats, and both nouns the card prints are already standing on
    // the other side of the table — so an uncastable spell here is the cost
    // and not "there is nothing for it to point at".
    let spell = in_hand(&engine, p0, altars_light()).expect("the spell is in hand");
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending());
    };
    assert!(
        !legal.castable.contains(&spell),
        "an empty pool pays no {{2}}{{W}}{{W}}, though a legal artifact and a \
         legal enchantment both stand across the table: {:?}",
        legal.castable
    );

    // Four of the eight Plains pay the first cast; the rest stay in the pool
    // for the second, because a pool survives inside one main phase (CR 500.5).
    cast_from_hand(&mut engine, p0, altars_light());
    let options = aim_at(&mut engine, p0, ring);
    assert!(
        options.contains(&ring) && options.contains(&chant),
        "\"target artifact or enchantment\" reaches either noun, on either \
         side of the table: {options:?}"
    );
    assert!(
        !options.contains(&elf),
        "the Elf is a creature and neither noun the card prints: {options:?}"
    );
    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, p1, quiet_artifact()).is_none(),
        "the artifact the spell named left the battlefield"
    );
    assert!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Exile(p1))
            .contains(&ring),
        "\"exile\": the card is in its owner's exile"
    );
    assert!(
        in_graveyard(&engine, p1, quiet_artifact()).is_none(),
        "and not in a graveyard, which is the word the card does not print"
    );
    assert!(
        on_battlefield(&engine, p1, their_enchantment()).is_some(),
        "one target, one card: the enchantment nobody named is untouched"
    );

    // The other half of the disjunction, read off the board the first cast
    // left behind: the exiled artifact is gone from the menu and the
    // enchantment is still on it.
    cast_from_hand(&mut engine, p0, altars_light());
    let options = aim_at(&mut engine, p0, chant);
    assert!(
        options.contains(&chant) && !options.contains(&ring),
        "the enchantment is the target still standing, and the exiled artifact \
         is no longer one: {options:?}"
    );
    assert!(
        !options.contains(&elf),
        "and a creature is still neither noun: {options:?}"
    );
    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, p1, their_enchantment()).is_none(),
        "the enchantment left the battlefield"
    );
    assert!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Exile(p1))
            .contains(&chant),
        "and the second noun is exiled the same way the first was"
    );
    assert!(
        in_graveyard(&engine, p1, their_enchantment()).is_none(),
        "never into a graveyard"
    );
    assert!(
        on_battlefield(&engine, p1, llanowar_elves()).is_some(),
        "the creature the spell could not name is still standing"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "two casts of {{2}}{{W}}{{W}} out of eight Plains spend the pool exactly"
    );
}
