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
const RECOLOR: u32 = 2;

// oracle_id = "65b3a40e-1d2b-4b24-996f-c935414ded7d"
fn chromatic_armor() -> CardIndex {
    card_index("65b3a40e-1d2b-4b24-996f-c935414ded7d")
}

#[track_caller]
fn cast_armor_on_bears(
    engine: &mut Engine<RegistryLookup>,
    bears: ObjectId,
    color: ManaColor,
) -> ObjectId {
    lk::float(
        engine,
        P0,
        &[
            (ManaColor::Colorless, 1),
            (ManaColor::White, 1),
            (ManaColor::Blue, 1),
        ],
    );
    cast_with_floating(engine, P0, chromatic_armor());
    lk::aim(engine, &[bears], &[]);
    pass_until(engine, |e| {
        matches!(e.pending(), Pending::ChooseColor { .. })
    });
    engine
        .apply(P0, PlayerAction::ChooseColor(color))
        .expect("choose a color as Chromatic Armor enters");
    pass_until(engine, stack_is_empty);
    lk::obj(engine, P0, chromatic_armor())
}

fn sleight_counters(engine: &Engine<RegistryLookup>, armor: ObjectId) -> u32 {
    u32::from(
        engine
            .state()
            .object(armor)
            .unwrap()
            .counters
            .get(counters::SLEIGHT),
    )
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
    let armor = cast_armor_on_bears(&mut engine, bears, ManaColor::Red);
    assert_eq!(
        sleight_counters(&engine, armor),
        1,
        "Chromatic Armor enters with a sleight counter on it"
    );

    // Cast Lightning Bolt (a red source) at enchanted Grizzly Bears: all 3 damage is prevented.
    lk::float(&mut engine, P0, &[(ManaColor::Red, 1)]);
    lk::cast_at(&mut engine, P0, index::LIGHTNING_BOLT, &[bears], &[]);

    assert!(
        on_battlefield(&engine, P0, index::GRIZZLY_BEARS).is_some(),
        "enchanted creature survives Lightning Bolt because damage from the chosen color is prevented"
    );
    assert_eq!(lk::damage_on(&engine, bears), 0);
}

#[test]
fn chromatic_armor_recolors_to_blue_and_prevents_blue_instead_of_red_damage() {
    let mut engine = lk::game(
        SEED,
        &[index::GRIZZLY_BEARS, index::PRODIGAL_SORCERER],
        &[],
        &[chromatic_armor(), index::LIGHTNING_BOLT],
        &[],
    );
    let bears = lk::obj(&engine, P0, index::GRIZZLY_BEARS);
    let armor = cast_armor_on_bears(&mut engine, bears, ManaColor::Red);

    lk::float(&mut engine, P0, &[(ManaColor::Colorless, 1)]);
    assert!(lk::offered(&engine, chromatic_armor(), P0, RECOLOR));
    engine
        .apply(
            P0,
            PlayerAction::ActivateAbility {
                source: armor,
                ability_index: RECOLOR,
            },
        )
        .expect("activate with X equal to the current sleight-counter count");
    assert!(
        !matches!(engine.pending(), Pending::ChooseNumber { .. }),
        "card-defined X is not a free numeric choice"
    );
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseColor { .. })
    });
    assert_eq!(
        lk::pool_total(&engine, P0),
        0,
        "the first activation pays {{1}}"
    );
    assert_eq!(
        sleight_counters(&engine, armor),
        2,
        "the counter is put on Chromatic Armor before its color choice"
    );
    engine
        .apply(P0, PlayerAction::ChooseColor(ManaColor::Blue))
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().object(armor).unwrap().chosen_color,
        Some(ManaColor::Blue),
        "the activation replaces the last chosen color"
    );

    lk::use_ability(&mut engine, P0, index::PRODIGAL_SORCERER, 0, &[bears], &[]);
    assert_eq!(
        lk::damage_on(&engine, bears),
        0,
        "a blue source's damage is prevented after choosing Blue"
    );

    lk::float(&mut engine, P0, &[(ManaColor::Red, 1)]);
    lk::cast_at(&mut engine, P0, index::LIGHTNING_BOLT, &[bears], &[]);
    assert_eq!(
        engine.state().object(bears).unwrap().zone,
        Zone::Graveyard,
        "red damage is no longer prevented after the last chosen color becomes Blue"
    );
}

#[test]
fn chromatic_armor_recounts_defined_x_and_refuses_an_unaffordable_activation() {
    let mut engine = lk::game(
        SEED,
        &[index::GRIZZLY_BEARS],
        &[],
        &[chromatic_armor()],
        &[],
    );
    let bears = lk::obj(&engine, P0, index::GRIZZLY_BEARS);
    let armor = cast_armor_on_bears(&mut engine, bears, ManaColor::Red);

    for (cost, color) in [(1, ManaColor::Blue), (2, ManaColor::Green)] {
        lk::float(&mut engine, P0, &[(ManaColor::Colorless, cost)]);
        assert!(lk::offered(&engine, chromatic_armor(), P0, RECOLOR));
        engine
            .apply(
                P0,
                PlayerAction::ActivateAbility {
                    source: armor,
                    ability_index: RECOLOR,
                },
            )
            .unwrap();
        assert!(!matches!(engine.pending(), Pending::ChooseNumber { .. }));
        pass_until(&mut engine, |e| {
            matches!(e.pending(), Pending::ChooseColor { .. })
        });
        assert_eq!(
            lk::pool_total(&engine, P0),
            0,
            "exactly {cost} mana is paid"
        );
        engine.apply(P0, PlayerAction::ChooseColor(color)).unwrap();
        pass_until(&mut engine, stack_is_empty);
        assert_eq!(sleight_counters(&engine, armor), cost + 1);
        assert_eq!(
            engine.state().object(armor).unwrap().chosen_color,
            Some(color)
        );
    }

    let before = engine.state().snapshot_hash();
    assert!(
        !lk::offered(&engine, chromatic_armor(), P0, RECOLOR),
        "with three counters and no mana, the {{3}} activation is not offered"
    );
    assert!(
        engine
            .apply(
                P0,
                PlayerAction::ActivateAbility {
                    source: armor,
                    ability_index: RECOLOR,
                },
            )
            .is_err(),
        "the direct action cannot bypass the unaffordable defined-X cost"
    );
    assert_eq!(engine.state().snapshot_hash(), before);
    assert_eq!(sleight_counters(&engine, armor), 3);
    assert_eq!(
        engine.state().object(armor).unwrap().chosen_color,
        Some(ManaColor::Green)
    );
}
