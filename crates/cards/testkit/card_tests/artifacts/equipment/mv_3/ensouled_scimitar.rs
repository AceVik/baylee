//! `cards/artifacts/equipment/mv_3/ensouled_scimitar.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn ensouled_scimitar() -> CardIndex {
    card_index("aa5865f5-aafc-4e46-995b-f005aeefde91")
}

/// Ensouled Scimitar: "{3}: This artifact becomes a 1/5 Spirit artifact
/// creature with flying until end of turn. Equipped creature gets +1/+5.
/// Equip {2}".
#[test]
fn ensouled_scimitar_gives_plus_one_plus_five() {
    equipped_gets(
        ensouled_scimitar(),
        2,
        llanowar_elves(),
        (1, 5),
        KeywordSet::EMPTY,
    );
}

#[test]
fn ensouled_scimitar_becomes_a_flying_spirit_until_end_of_turn() {
    use baylee_core::generated::subtypes::creature::SPIRIT;
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), forest(), ensouled_scimitar()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let sword = on_battlefield(&engine, p0, ensouled_scimitar()).expect("the Scimitar");
    assert!(
        !types(&engine, sword).contains(TypeSet::CREATURE),
        "no creature yet"
    );
    tap_all_mana(&mut engine, p0);
    // Ability 0 is the Equip, 1 the animation.
    activate(&mut engine, p0, ensouled_scimitar(), 1);
    pass_until(&mut engine, stack_is_empty);
    let t = types(&engine, sword);
    assert!(
        t.contains(TypeSet::CREATURE) && t.contains(TypeSet::ARTIFACT),
        "{t:?}"
    );
    assert_eq!(pt(&engine, sword), (1, 5));
    assert!(keywords(&engine, sword).contains(KeywordSet::FLYING));
    assert!(subtypes_of(&engine, sword).contains(SPIRIT));
    reach_their_main_phase(&mut engine, p1);
    assert!(
        !types(&engine, sword).contains(TypeSet::CREATURE),
        "until end of turn"
    );
}
