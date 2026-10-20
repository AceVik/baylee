//! `cards/enchantments/auras/mv_3/chromatic_armor.rs`, played.
//!
//! Oracle:
//! - "Enchant creature"
//! - "As this Aura enters, choose a color."
//! - "This Aura enters with a sleight counter on it."
//! - "Prevent all damage that would be dealt to enchanted creature by sources of the last chosen color."
//! - "{X}: Put a sleight counter on this Aura and choose a color. X is the number of sleight counters on this Aura."

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use baylee_cards_dsl::counters;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);

// oracle_id = "65b3a40e-1d2b-4b24-996f-c935414ded7d"
fn chromatic_armor() -> CardIndex {
    card_index("65b3a40e-1d2b-4b24-996f-c935414ded7d")
}

#[test]
fn chromatic_armor_enters_with_sleight_counter_and_prevents_damage_from_chosen_color() {
    let mut engine = lk::game(
        SEED,
        &[index::GRIZZLY_BEARS],
        &[],
        &[chromatic_armor(), index::LIGHTNING_BOLT],
        &[],
    );
    let bears = lk::obj(&engine, P0, index::GRIZZLY_BEARS);

    lk::float(
        &mut engine,
        P0,
        &[
            (ManaColor::Colorless, 1),
            (ManaColor::White, 1),
            (ManaColor::Blue, 1),
        ],
    );
    cast_with_floating(&mut engine, P0, chromatic_armor());
    lk::aim(&mut engine, &[bears], &[]);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseColor { .. })
    });
    engine
        .apply(P0, PlayerAction::ChooseColor(ManaColor::Red))
        .expect("choose Red as Chromatic Armor enters");
    pass_until(&mut engine, stack_is_empty);

    let armor = on_battlefield(&engine, P0, chromatic_armor()).expect("on battlefield");
    assert_eq!(
        engine
            .state()
            .object(armor)
            .unwrap()
            .counters
            .get(counters::SLEIGHT),
        1,
        "Chromatic Armor enters with a sleight counter on it"
    );

    // Cast Lightning Bolt (a red source) at enchanted Grizzly Bears: all 3 damage is prevented.
    lk::float(&mut engine, P0, &[(ManaColor::Red, 1)]);
    cast_with_floating(&mut engine, P0, index::LIGHTNING_BOLT);
    lk::aim(&mut engine, &[bears], &[]);
    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, P0, index::GRIZZLY_BEARS).is_some(),
        "enchanted creature survives Lightning Bolt because damage from the chosen color is prevented"
    );
    assert_eq!(lk::damage_on(&engine, bears), 0);
}
