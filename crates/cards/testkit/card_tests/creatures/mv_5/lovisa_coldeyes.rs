//! `cards/creatures/mv_5/lovisa_coldeyes.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Lovisa Coldeyes` is a 3/3 legendary Human costing `{3}{R}{R}` under `Coverage::Implemented`.
/// It prints "Each creature that's a Barbarian, a Warrior, or a Berserker gets +2/+2 and has haste."
/// On a board with `Barbarian Riftcutter` (a Barbarian) and `Llanowar Elves` (an Elf Druid), Lovisa grants
/// +2/+2 and haste to the Barbarian, while leaving the Elf and Lovisa herself unaffected.
#[test]
fn lovisa_coldeyes_buffs_barbarians_warriors_and_berserkers_only() {
    let p0 = PlayerId::new(0);
    let riftcutter_card = card_index("e9612e4c-1527-4255-9f2f-b8d8cee65cc6");
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[lovisa_coldeyes(), riftcutter_card, llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let lovisa = on_battlefield(&engine, p0, lovisa_coldeyes()).expect("Lovisa is present");
    let barbarian =
        on_battlefield(&engine, p0, riftcutter_card).expect("Barbarian Riftcutter is present");
    let elf = on_battlefield(&engine, p0, llanowar_elves()).expect("Llanowar Elves is present");

    // Barbarian gets +2/+2 (printed 3/3 becomes 5/5) and haste.
    assert_eq!(
        pt(&engine, barbarian),
        (5, 5),
        "Barbarian creature gets +2/+2"
    );
    assert!(
        keywords(&engine, barbarian).contains(KeywordSet::HASTE),
        "Barbarian creature gains haste"
    );

    // Non-matching creature (Elf Druid) remains untouched.
    assert_eq!(pt(&engine, elf), (1, 1), "Llanowar Elves remains 1/1");
    assert!(
        !keywords(&engine, elf).contains(KeywordSet::HASTE),
        "Llanowar Elves does not gain haste"
    );

    // Lovisa herself is only a Human and does not buff herself.
    assert_eq!(pt(&engine, lovisa), (3, 3), "Lovisa Coldeyes remains 3/3");
    assert!(
        !keywords(&engine, lovisa).contains(KeywordSet::HASTE),
        "Lovisa Coldeyes does not have haste"
    );
}
