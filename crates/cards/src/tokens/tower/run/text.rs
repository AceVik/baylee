//! Original table-facing ability sentences, indexed exactly as the executable DSL.
use super::{RUN_RELICS, TokenDef};

const UPKEEP_LIFE: &str = "At the beginning of your upkeep, you gain 1 life.";
const UPKEEP_SCRY: &str = "At the beginning of your upkeep, scry 1.";
const UPKEEP_PLANT: &str =
    "At the beginning of your upkeep, create a 1/1 green Plant creature token.";
const LOOT: &str =
    "{2}: Draw a card, then discard a card. Activate only once each turn from the command zone.";
const DRAW_LIFE: &str = "Whenever you draw a card, you gain 1 life.";
const DEATH_LIFE: &str = "Whenever a creature you control dies, you gain 1 life.";
const LIFE_DRAIN: &str = "Whenever you gain life, each opponent loses 1 life.";
const UPKEEP_CHARGE: &str = "At the beginning of your upkeep, put a charge counter on this item.";
const SPELL_LIFE: &str = "Whenever you cast a noncreature spell, you gain 1 life.";
const SPELL_PLANT: &str =
    "Whenever you cast a noncreature spell, create a 1/1 green Plant creature token.";

const EXTRA: [[&str; 3]; 18] = [
    [
        UPKEEP_LIFE,
        "Whenever a creature you control enters, you gain 1 life.",
        LIFE_DRAIN,
    ],
    [UPKEEP_SCRY, LOOT, DRAW_LIFE],
    [
        UPKEEP_SCRY,
        "{1}: Add one mana of any color. Activate only once each turn from the command zone.",
        "Whenever you cast your first noncreature spell each turn, draw a card.",
    ],
    [
        "Whenever a creature token you control enters, you gain 1 life.",
        "Creature tokens you control get +1/+1.",
        "Creature tokens you control have trample.",
    ],
    [
        DRAW_LIFE,
        "{2}, Pay 2 life: Draw a card. Activate only once each turn from the command zone.",
        "At the beginning of your upkeep, draw a card.",
    ],
    [
        UPKEEP_PLANT,
        "Creature tokens you control get +0/+1.",
        LIFE_DRAIN,
    ],
    [UPKEEP_CHARGE, DRAW_LIFE, LIFE_DRAIN],
    [
        DRAW_LIFE,
        "At the beginning of your upkeep, you gain 2 life.",
        "Whenever you draw a card, each opponent loses 1 life.",
    ],
    [
        UPKEEP_LIFE,
        "Creature tokens you control get +0/+1.",
        "Creature tokens you control have haste.",
    ],
    [
        "Creature tokens you control have vigilance.",
        "Creature tokens you control have trample.",
        "Creature tokens you control have lifelink.",
    ],
    [
        DEATH_LIFE,
        UPKEEP_PLANT,
        "Creature tokens you control have haste.",
    ],
    [
        UPKEEP_PLANT,
        DEATH_LIFE,
        "Creature tokens you control get +1/+1.",
    ],
    [UPKEEP_SCRY, SPELL_LIFE, SPELL_PLANT],
    [UPKEEP_SCRY, LOOT, DRAW_LIFE],
    [
        DEATH_LIFE,
        UPKEEP_PLANT,
        "Whenever a nontoken creature you control dies, draw a card.",
    ],
    [UPKEEP_CHARGE, SPELL_LIFE, SPELL_PLANT],
    [
        "Creatures you control have vigilance.",
        "Creatures you control have lifelink.",
        "Creatures you control have hexproof.",
    ],
    [
        UPKEEP_LIFE,
        UPKEEP_PLANT,
        "Creature tokens you control get +1/+1.",
    ],
];

const BASE: [usize; 18] = [0, 0, 0, 1, 1, 1, 2, 1, 1, 1, 2, 2, 1, 2, 1, 2, 1, 0];

/// A sentence exists for every executable ability, including paid command actions.
#[must_use]
pub fn ability_text(token: &TokenDef, index: u32) -> Option<&'static str> {
    let index = usize::try_from(index).ok()?;
    if index >= token.abilities.len() {
        return None;
    }
    if let Some(text) = super::runic::ability_text(token, index) {
        return Some(text);
    }
    let (family, tier) = RUN_RELICS.iter().enumerate().find_map(|(family, tiers)| {
        tiers
            .iter()
            .position(|candidate| std::ptr::eq(token, *candidate))
            .map(|tier| (family, tier))
    })?;
    if index >= BASE[family] {
        return EXTRA[family].get(index - BASE[family]).copied();
    }
    base(family, tier, index)
}

fn base(family: usize, tier: usize, index: usize) -> Option<&'static str> {
    use super::super::{GROWTH_RELICS, LIFE_RELICS};
    let old = match family {
        3 => Some(
            [
                &super::super::TOWER_GROVE_COMMON,
                &super::super::TOWER_GROVE_UNCOMMON,
                &super::super::TOWER_GROVE_RARE,
                &super::super::TOWER_GROVE_MYTHIC,
            ][tier],
        ),
        4 | 5 => Some(LIFE_RELICS[family - 4][tier]),
        8 | 9 => Some(GROWTH_RELICS[family - 8][tier]),
        _ => None,
    };
    if let Some(old) = old {
        return super::super::ability_text(old, u32::try_from(index).ok()?);
    }
    Some(match (family, index) {
        (6, 0) => "Whenever you gain life, put a charge counter on this item.",
        (6, 1) => [
            "Remove two charge counters: This item deals 1 damage to any target. Activate only once each turn from the command zone.",
            "Remove two charge counters: This item deals 2 damage to any target. Activate only once each turn from the command zone.",
            "Remove two charge counters: This item deals 3 damage to any target. Activate only once each turn from the command zone.",
            "Remove two charge counters: This item deals 4 damage to any target. Activate from the command zone.",
        ][tier],
        (7, 0) => [
            "{3}, Pay 2 life: Draw a card. Activate only once each turn from the command zone.",
            "{2}, Pay 2 life: Draw a card. Activate only once each turn from the command zone.",
            "{1}, Pay 2 life: Draw a card. Activate only once each turn from the command zone.",
            "{1}, Pay 2 life: Draw a card. Activate from the command zone.",
        ][tier],
        (10, 0) => "Whenever a creature token you control dies, put a charge counter on this item.",
        (10, 1) => [
            "Remove three charge counters: Add {C}{C}. Activate only once each turn from the command zone.",
            "Remove two charge counters: Add {C}{C}. Activate only once each turn from the command zone.",
            "Remove a charge counter: Add {C}{C}. Activate only once each turn from the command zone.",
            "Remove a charge counter: Add {C}{C}{C}. Activate from the command zone.",
        ][tier],
        (11, 0) => {
            "Whenever a creature token you control enters, put a charge counter on this item."
        }
        (11, 1) => [
            "Remove three charge counters: Add {G}. Activate only once each turn from the command zone.",
            "Remove two charge counters: Add {G}. Activate only once each turn from the command zone.",
            "Remove two charge counters: Add {G}. Activate from the command zone.",
            "Remove a charge counter: Add {G}. Activate from the command zone.",
        ][tier],
        (12, 0) => [
            "Whenever you cast a noncreature spell, each opponent loses 1 life.",
            "Whenever you cast a noncreature spell, each opponent loses 2 life.",
            "Whenever you cast a noncreature spell, each opponent loses 3 life.",
            "Whenever you cast a noncreature spell, each opponent loses 4 life.",
        ][tier],
        (13, 0) => "Whenever you draw a card, put a charge counter on this item.",
        (13, 1) => [
            "Remove three charge counters: Scry 1. Activate from the command zone.",
            "Remove three charge counters: Scry 2. Activate from the command zone.",
            "Remove three charge counters: Scry 3. Activate from the command zone.",
            "Remove three charge counters: Scry 4. Activate from the command zone.",
        ][tier],
        (14, 0) => [
            "Whenever a nontoken creature you control dies, each opponent loses 1 life.",
            "Whenever a nontoken creature you control dies, each opponent loses 2 life.",
            "Whenever a nontoken creature you control dies, each opponent loses 3 life.",
            "Whenever a nontoken creature you control dies, each opponent loses 4 life.",
        ][tier],
        (15, 0) => "Whenever you cast a noncreature spell, put a charge counter on this item.",
        (15, 1) => [
            "Remove three charge counters: Add one mana of any color. Activate only once each turn from the command zone.",
            "Remove two charge counters: Add one mana of any color. Activate only once each turn from the command zone.",
            "Remove a charge counter: Add one mana of any color. Activate only once each turn from the command zone.",
            "Remove a charge counter: Add one mana of any color. Activate from the command zone.",
        ][tier],
        (16, 0) => [
            "Creatures you control get +0/+1.",
            "Creatures you control get +0/+2.",
            "Creatures you control get +0/+3.",
            "Creatures you control get +0/+4.",
        ][tier],
        _ => return None,
    })
}
