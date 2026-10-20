//! `cards/enchantments/auras/mv_3/forbidden_lore.rs`, played.
//!
//! Oracle:
//! - "Enchant land"
//! - "Enchanted land has \"{T}: Target creature gets +2/+1 until end of turn.\""

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);

fn forbidden_lore() -> CardIndex {
    card_index("d2853bad-a751-45ee-8269-24b0cdbc5c6b")
}

#[test]
fn forbidden_lore_grants_enchanted_land_tap_to_pump_creature() {
    let mut engine = lk::game(
        SEED,
        &[index::FOREST, index::GRIZZLY_BEARS],
        &[],
        &[forbidden_lore()],
        &[],
    );
    let land = lk::obj(&engine, P0, index::FOREST);
    let bears = lk::obj(&engine, P0, index::GRIZZLY_BEARS);

    lk::float(
        &mut engine,
        P0,
        &[(ManaColor::Colorless, 2), (ManaColor::Green, 1)],
    );
    lk::cast_at(&mut engine, P0, forbidden_lore(), &[land], &[]);

    engine
        .apply(
            P0,
            PlayerAction::ActivateAbility {
                source: land,
                ability_index: crate::choice::GRANTED_ABILITY,
            },
        )
        .unwrap();
    lk::aim(&mut engine, &[bears], &[]);
    lk::settle(&mut engine);

    assert!(lk::tapped(&engine, land));
    assert_eq!(
        engine
            .state()
            .object(bears)
            .unwrap()
            .characteristics()
            .power,
        Some(4)
    );
    assert_eq!(
        engine
            .state()
            .object(bears)
            .unwrap()
            .characteristics()
            .toughness,
        Some(3)
    );
}
