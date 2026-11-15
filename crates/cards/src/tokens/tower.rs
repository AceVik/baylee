//! Original Baylee Tower supplies. No card art or external card identity.
//!
//! Potion definitions share the stable object/ability registry with tokens,
//! but the engine creates them as command-zone emblems, never permanents.

use baylee_cards_dsl::effect::CounterKind;
use baylee_cards_dsl::{ActivationZone, Cost, CostPart, Effect, TokenDef, activated, mana_ability};
use baylee_cards_dsl::{Amount, PlayerRel, StepKind, Trigger, triggered};
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::subtypes::creature;
use baylee_core::preset::TowerSupply;
use baylee_core::types::TypeSet;

#[path = "tower/amber.rs"]
mod amber;
#[path = "tower/chalice.rs"]
mod chalice;
#[path = "tower/coil.rs"]
mod coil;
#[path = "tower/hearth.rs"]
mod hearth;
#[path = "tower/life.rs"]
mod life;
pub use amber::*;
pub use chalice::*;
pub use coil::*;
pub use hearth::*;
pub use life::LIFE_RELICS;

const CHARGE: Cost = Cost {
    parts: &[CostPart::RemoveCounterSelf {
        kind: CounterKind::Charge,
        n: 1,
    }],
    ..Cost::FREE
};

/// Original healing emblem: each activation spends one paid charge.
pub static TOWER_HEALING_POTION: TokenDef = TokenDef {
    name: "Tower Healing Potion",
    abilities: &[activated!(
        CHARGE,
        &[Effect::gain_life(5)],
        zone = ActivationZone::Command
    )],
    ..TokenDef::DEFAULT
};

/// Original mana emblem: the existing color question chooses the mana.
pub static TOWER_MANA_POTION: TokenDef = TokenDef {
    name: "Tower Mana Potion",
    abilities: &[mana_ability!(
        CHARGE,
        &[Effect::mana_of_any_color()],
        zone = ActivationZone::Command
    )],
    ..TokenDef::DEFAULT
};

/// Original starting Plant, subject to ordinary creature and token rules.
pub static TOWER_SEEDLING: TokenDef = TokenDef {
    name: "Tower Seedling",
    colors: ColorSet::from_slice(&[Color::Green]),
    types: TypeSet::CREATURE,
    subtypes: &[creature::PLANT],
    power: Some(1),
    toughness: Some(1),
    ..TokenDef::DEFAULT
};

/// Original Common Grove: one Plant at the beginning of your upkeep.
pub static TOWER_GROVE_COMMON: TokenDef = TokenDef {
    name: "Living Grove — Common",
    types: TypeSet::ARTIFACT,
    abilities: &[triggered!(
        Trigger::StepBegin {
            step: StepKind::Upkeep,
            whose: PlayerRel::You
        },
        &[Effect::CreateTokensFor {
            player: PlayerRel::You,
            token: &TOWER_SEEDLING,
            amount: Amount::Fixed(1)
        }]
    )],
    ..TokenDef::DEFAULT
};

/// Original Uncommon Grove: two Plants at the beginning of your upkeep.
pub static TOWER_GROVE_UNCOMMON: TokenDef = TokenDef {
    name: "Living Grove — Uncommon",
    types: TypeSet::ARTIFACT,
    abilities: &[triggered!(
        Trigger::StepBegin {
            step: StepKind::Upkeep,
            whose: PlayerRel::You
        },
        &[Effect::CreateTokensFor {
            player: PlayerRel::You,
            token: &TOWER_SEEDLING,
            amount: Amount::Fixed(2)
        }]
    )],
    ..TokenDef::DEFAULT
};

/// Original Rare Grove: three Plants at the beginning of your upkeep.
pub static TOWER_GROVE_RARE: TokenDef = TokenDef {
    name: "Living Grove — Rare",
    types: TypeSet::ARTIFACT,
    abilities: &[triggered!(
        Trigger::StepBegin {
            step: StepKind::Upkeep,
            whose: PlayerRel::You
        },
        &[Effect::CreateTokensFor {
            player: PlayerRel::You,
            token: &TOWER_SEEDLING,
            amount: Amount::Fixed(3)
        }]
    )],
    ..TokenDef::DEFAULT
};

/// Original Mythic Grove: four Plants at the beginning of your upkeep.
pub static TOWER_GROVE_MYTHIC: TokenDef = TokenDef {
    name: "Living Grove — Mythic Rare",
    types: TypeSet::ARTIFACT,
    abilities: &[triggered!(
        Trigger::StepBegin {
            step: StepKind::Upkeep,
            whose: PlayerRel::You
        },
        &[Effect::CreateTokensFor {
            player: PlayerRel::You,
            token: &TOWER_SEEDLING,
            amount: Amount::Fixed(4)
        }]
    )],
    ..TokenDef::DEFAULT
};

/// Definition used by an authoritative custom preset.
#[must_use]
pub const fn definition(supply: TowerSupply) -> &'static TokenDef {
    match supply {
        TowerSupply::HealingPotion => &TOWER_HEALING_POTION,
        TowerSupply::ManaPotion => &TOWER_MANA_POTION,
        TowerSupply::Seedling => &TOWER_SEEDLING,
    }
}

/// Original potion identity for UI and narration; other tokens are not supplies.
#[must_use]
pub fn potion_kind(id: u16) -> Option<TowerSupply> {
    let token = super::by_token_id(id)?;
    if std::ptr::eq(token, &raw const TOWER_HEALING_POTION) {
        Some(TowerSupply::HealingPotion)
    } else if std::ptr::eq(token, &raw const TOWER_MANA_POTION) {
        Some(TowerSupply::ManaPotion)
    } else {
        None
    }
}

/// Original rules text, not an Oracle quotation.
pub(super) fn ability_text(token: &TokenDef, index: u32) -> Option<&'static str> {
    if let Some(text) = life::ability_text(token, index) {
        return Some(text);
    }
    if index != 0 {
        return None;
    }
    for (definition, text) in [
        (
            &TOWER_GROVE_COMMON,
            "At the beginning of your upkeep, create a 1/1 green Plant creature token.",
        ),
        (
            &TOWER_GROVE_UNCOMMON,
            "At the beginning of your upkeep, create two 1/1 green Plant creature tokens.",
        ),
        (
            &TOWER_GROVE_RARE,
            "At the beginning of your upkeep, create three 1/1 green Plant creature tokens.",
        ),
        (
            &TOWER_GROVE_MYTHIC,
            "At the beginning of your upkeep, create four 1/1 green Plant creature tokens.",
        ),
    ] {
        if std::ptr::eq(token, definition) {
            return Some(text);
        }
    }
    if std::ptr::eq(token, &raw const TOWER_HEALING_POTION) {
        Some("Remove a charge: You gain 5 life. Activate from the command zone.")
    } else if std::ptr::eq(token, &raw const TOWER_MANA_POTION) {
        Some("Remove a charge: Add one mana of any color. Activate from the command zone.")
    } else {
        None
    }
}
