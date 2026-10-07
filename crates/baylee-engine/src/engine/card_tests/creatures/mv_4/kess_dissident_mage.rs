//! `cards/creatures/mv_4/kess_dissident_mage.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Kess, Dissident Mage prints two sentences and the DSL carries the first
/// one: `Flying`, with the once-per-turn graveyard cast left to the
/// `// NOT SUPPORTED` line. What this plays is that half — she is cast off
/// `{1}{U}{B}{R}`, resolves, and stands as the printed 3/4 with flying
/// projected onto her by the keyword itself and by nothing else on the
/// board. The Llanowar Elves beside her are the counter-half: a projection
/// that had lost flying's subject and handed it to every creature under the
/// seat would read exactly the same on Kess alone, and reads wrong here.
#[test]
fn kess_resolves_as_a_flying_three_four_and_grants_flying_to_nobody_else() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(97, island())
        .battlefield(
            0,
            &[island(), swamp(), mountain(), mountain(), llanowar_elves()],
        )
        .hand(0, &[kess_dissident_mage()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let elves = on_battlefield(&engine, p0, llanowar_elves()).expect("the Elves are out");
    assert!(
        !keywords(&engine, elves).contains(KeywordSet::FLYING),
        "the ground-bound bystander flies before she lands"
    );

    // The four lands pay {1}{U}{B}{R} exactly. The Elf is kept back so that
    // "exactly" stays true: it makes a fifth mana that would still be
    // floating at the assertion below.
    tap_all_mana_but(&mut engine, p0, Some(llanowar_elves()));
    cast_with_floating(&mut engine, p0, kess_dissident_mage());
    pass_until(&mut engine, stack_is_empty);

    let kess = on_battlefield(&engine, p0, kess_dissident_mage()).expect("she resolved");
    assert_eq!(pt(&engine, kess), (3, 4), "the body the card prints");
    let types = engine
        .state()
        .object(kess)
        .expect("she is still an object")
        .characteristics()
        .types;
    assert!(
        types.contains(TypeSet::CREATURE),
        "a legendary Human Wizard is a creature: {types:?}"
    );
    assert!(
        keywords(&engine, kess).contains(KeywordSet::FLYING),
        "\"Flying\" is the half of the card the DSL carries"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "{{1}}{{U}}{{B}}{{R}} was paid out of the pool, not left floating"
    );
    assert!(
        !keywords(&engine, elves).contains(KeywordSet::FLYING),
        "and flying stayed where it was printed: the Elves are still on the ground"
    );
}
