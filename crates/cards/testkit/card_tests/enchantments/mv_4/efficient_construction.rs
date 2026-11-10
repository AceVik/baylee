//! `cards/enchantments/mv_4/efficient_construction.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn efficient_construction() -> CardIndex {
    card_index("5af48f87-7b94-44de-90e3-91f10ced00d3")
}

/// "Whenever you cast an artifact spell, create a 1/1 colorless Thopter
/// artifact creature token with flying": one Thopter for the artifact spell,
/// none for a creature spell.
#[test]
fn efficient_construction_makes_a_thopter_for_an_artifact_spell_only() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[efficient_construction()])
        .hand(0, &[grizzly_bears(), sol_ring()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    float(&mut engine, p0, ManaColor::Green, 2);
    cast_with_floating(&mut engine, p0, grizzly_bears());
    pass_until(&mut engine, stack_is_empty);
    assert!(
        tokens_of(&engine, p0).is_empty(),
        "a creature spell makes none"
    );

    float(&mut engine, p0, ManaColor::Green, 1);
    cast_with_floating(&mut engine, p0, sol_ring());
    pass_until(&mut engine, stack_is_empty);
    let tokens = tokens_of(&engine, p0);
    assert_eq!(tokens.len(), 1, "one Thopter");
    let chars = engine
        .state()
        .object(tokens[0])
        .expect("token")
        .characteristics();
    assert_eq!((chars.power, chars.toughness), (Some(1), Some(1)));
    assert!(
        chars
            .keywords
            .contains(baylee_cards_dsl::KeywordSet::FLYING)
    );
    assert!(chars.types.contains(TypeSet::ARTIFACT) && chars.types.contains(TypeSet::CREATURE));
    assert!(chars.colors.is_empty(), "colorless");
}
