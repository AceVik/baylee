//! `cards/enchantments/mv_4/aquatic_incursion.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn aquatic_incursion() -> CardIndex {
    card_index("e4212b05-d397-49fb-af8b-9e52a60e6d1e")
}

/// "When this enchantment enters, create two 1/1 blue Merfolk creature tokens
/// with hexproof." Then "{3}{U}: Target Merfolk can't be blocked this turn.":
/// the menu holds the Merfolk and not the Bears beside them, and the target
/// gains unblockable.
#[test]
fn aquatic_incursion_makes_two_hexproof_merfolk_and_makes_one_unblockable() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[grizzly_bears()])
        .hand(0, &[aquatic_incursion()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    float(&mut engine, p0, ManaColor::Blue, 4);
    cast_with_floating(&mut engine, p0, aquatic_incursion());
    pass_until(&mut engine, stack_is_empty);
    let merfolk = tokens_of(&engine, p0);
    assert_eq!(merfolk.len(), 2, "two Merfolk");
    for &m in &merfolk {
        let chars = engine.state().object(m).expect("token").characteristics();
        assert_eq!((chars.power, chars.toughness), (Some(1), Some(1)));
        assert!(chars.colors.contains(Color::Blue), "blue");
        assert!(
            chars
                .keywords
                .contains(baylee_cards_dsl::KeywordSet::HEXPROOF),
            "hexproof"
        );
        assert!(
            chars
                .subtypes
                .contains(baylee_core::generated::subtypes::creature::MERFOLK)
        );
    }

    float(&mut engine, p0, ManaColor::Blue, 4);
    activate(&mut engine, p0, aquatic_incursion(), 1);
    let menu = aim_at(&mut engine, p0, merfolk[0]);
    let bears = on_battlefield(&engine, p0, grizzly_bears()).expect("seated");
    assert!(menu.contains(&merfolk[0]) && menu.contains(&merfolk[1]));
    assert!(!menu.contains(&bears), "Bears are no Merfolk");
    pass_until(&mut engine, stack_is_empty);
    let unblockable = baylee_cards_dsl::KeywordSet::UNBLOCKABLE;
    assert!(
        keywords(&engine, merfolk[0]).contains(unblockable),
        "the target"
    );
    assert!(
        !keywords(&engine, merfolk[1]).contains(unblockable),
        "only the target"
    );
}
