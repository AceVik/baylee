//! `cards/enchantments/auras/mv_1/fylgja.rs`, played.
//!
//! Oracle:
//! - "Enchant creature"
//! - "This Aura enters with four healing counters on it."
//! - "Remove a healing counter from this Aura: Prevent the next 1 damage that would be dealt to enchanted creature this turn."
//! - "{2}{W}: Put a healing counter on this Aura."

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);

fn fylgja() -> CardIndex {
    card_index("df0e5dd8-65e0-474c-ac32-6e0f6f16b8e6")
}

#[test]
fn fylgja_enters_with_four_healing_counters_prevents_damage_and_recharges() {
    let mut engine = lk::game(
        SEED,
        &[index::SERRA_ANGEL],
        &[],
        &[fylgja(), index::LIGHTNING_BOLT],
        &[],
    );
    let angel = lk::obj(&engine, P0, index::SERRA_ANGEL);

    lk::float(&mut engine, P0, &[(ManaColor::White, 1)]);
    lk::cast_at(&mut engine, P0, fylgja(), &[angel], &[]);
    let aura = lk::obj(&engine, P0, fylgja());

    assert_eq!(
        engine
            .state()
            .object(aura)
            .unwrap()
            .counters
            .get(baylee_cards_dsl::counters::HEALING),
        4
    );

    // Activate once so 1 of Lightning Bolt's 3 damage is prevented on the 4/4 Serra Angel.
    lk::use_ability(&mut engine, P0, fylgja(), 1, &[angel], &[]);
    assert_eq!(
        engine
            .state()
            .object(aura)
            .unwrap()
            .counters
            .get(baylee_cards_dsl::counters::HEALING),
        3
    );

    lk::float(&mut engine, P0, &[(ManaColor::Red, 1)]);
    lk::cast_at(&mut engine, P0, index::LIGHTNING_BOLT, &[angel], &[]);
    assert_eq!(
        lk::damage_on(&engine, angel),
        2,
        "1 of Lightning Bolt's 3 damage was prevented, leaving 2 damage on Serra Angel"
    );

    lk::float(
        &mut engine,
        P0,
        &[(ManaColor::Colorless, 2), (ManaColor::White, 1)],
    );
    lk::use_ability(&mut engine, P0, fylgja(), 2, &[], &[]);
    assert_eq!(
        engine
            .state()
            .object(aura)
            .unwrap()
            .counters
            .get(baylee_cards_dsl::counters::HEALING),
        4
    );
}
