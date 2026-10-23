//! `cards/enchantments/auras/mv_2/lightning_reflexes.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use crate::turn::Step;
use baylee_cards_dsl::KeywordSet;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);

// oracle_id = "2b0c82f1-078a-4e8b-8e43-407f5c5d8549"
fn lightning_reflexes() -> CardIndex {
    card_index("2b0c82f1-078a-4e8b-8e43-407f5c5d8549")
}

#[test]
fn lightning_reflexes_grants_plus_1_plus_0_and_first_strike_to_enchanted_creature() {
    let mut engine = lk::game(
        SEED,
        &[index::GRIZZLY_BEARS],
        &[],
        &[lightning_reflexes()],
        &[],
    );
    let bears = lk::obj(&engine, P0, index::GRIZZLY_BEARS);

    lk::float(
        &mut engine,
        P0,
        &[(ManaColor::Colorless, 1), (ManaColor::Red, 1)],
    );
    cast_with_floating(&mut engine, P0, lightning_reflexes());
    lk::aim(&mut engine, &[bears], &[]);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(pt(&engine, bears), (3, 2));
    assert!(
        engine
            .state()
            .object(bears)
            .unwrap()
            .characteristics()
            .keywords
            .contains(KeywordSet::FIRST_STRIKE)
    );
}

/// Flash is left off because the sacrifice rider that goes with it is not
/// supported: the Aura is castable in the main phase and not in the end step.
#[test]
fn lightning_reflexes_is_castable_only_at_sorcery_speed() {
    let mut engine = lk::game(
        SEED,
        &[index::GRIZZLY_BEARS, mountain(), mountain()],
        &[],
        &[lightning_reflexes()],
        &[],
    );
    lk::float(
        &mut engine,
        P0,
        &[(ManaColor::Colorless, 1), (ManaColor::Red, 1)],
    );
    assert!(
        lk::castable(&engine, P0, lightning_reflexes()),
        "castable in the main phase with its mana floating"
    );

    lk::to_step(&mut engine, P0, Step::End);
    lk::float(
        &mut engine,
        P0,
        &[(ManaColor::Colorless, 1), (ManaColor::Red, 1)],
    );
    assert!(
        !lk::castable(&engine, P0, lightning_reflexes()),
        "not castable in the end step without flash"
    );
}
