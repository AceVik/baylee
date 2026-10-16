//! `cards/artifacts/mv_4/gruul_war_plow.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn gruul_war_plow() -> CardIndex {
    card_index("6dba119e-1b08-42e0-8d0a-b606bd90af02")
}

/// "Creatures you control have trample." Mine do, theirs do not.
#[test]
fn only_my_creatures_have_trample() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[gruul_war_plow(), llanowar_elves()])
        .battlefield(1, &[gray_ogre()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let elf = on_battlefield(&engine, p0, llanowar_elves()).unwrap();
    let ogre = on_battlefield(&engine, p1, gray_ogre()).unwrap();
    assert!(keywords(&engine, elf).contains(KeywordSet::TRAMPLE));
    assert!(!keywords(&engine, ogre).contains(KeywordSet::TRAMPLE));
}

/// "{1}{R}{G}: This artifact becomes a 4/4 Juggernaut artifact creature until
/// end of turn." Before: no creature. After: a 4/4 Juggernaut that, being a
/// creature I control, has trample too. At the next turn it is an artifact
/// again.
#[test]
fn it_becomes_a_4_4_juggernaut_until_end_of_turn() {
    use baylee_core::generated::subtypes;
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[gruul_war_plow(), mountain(), forest(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let plow = on_battlefield(&engine, p0, gruul_war_plow()).unwrap();
    let chars = |e: &Engine<RegistryLookup>| e.state().object(plow).unwrap().characteristics().clone();
    assert!(!chars(&engine).types.contains(TypeSet::CREATURE));

    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, gruul_war_plow(), 1);
    pass_until(&mut engine, stack_is_empty);

    let c = chars(&engine);
    assert!(c.types.contains(TypeSet::CREATURE), "a creature now");
    assert!(c.types.contains(TypeSet::ARTIFACT), "still an artifact");
    assert!(c.subtypes.contains(subtypes::creature::JUGGERNAUT));
    assert_eq!(pt(&engine, plow), (4, 4));
    assert!(
        c.keywords.contains(KeywordSet::TRAMPLE),
        "a creature I control, so the Plow's own static covers it"
    );

    reach_their_main_phase(&mut engine, PlayerId::new(1));
    assert!(
        !chars(&engine).types.contains(TypeSet::CREATURE),
        "until end of turn"
    );
}
