//! Generation-five independent equipment. Earlier artifact definitions are frozen.
//!
//! Every source is an emblem: triggers/statics use ordinary emblem rules and
//! activations explicitly use the command zone. No item executes an external
//! callback, changes a card rule, or borrows a removable artifact identity.
use baylee_cards_dsl::prelude::*;

mod arcane;
mod growth;
mod opening;
pub mod runic;
mod text;
mod vitality;
pub use arcane::*;
pub use growth::*;
pub use opening::*;
pub use runic::*;
pub use text::ability_text;
pub use vitality::*;

const fn upkeep(effects: &'static [Effect]) -> AbilityDef {
    triggered!(
        Trigger::StepBegin {
            step: StepKind::Upkeep,
            whose: PlayerRel::You
        },
        effects
    )
}

const fn paid(cost: Cost, effects: &'static [Effect], limit: ActivationLimit) -> AbilityDef {
    activated!(cost, effects, zone = ActivationZone::Command, limit = limit)
}

const fn mana(cost: Cost, effects: &'static [Effect], limit: ActivationLimit) -> AbilityDef {
    mana_ability!(cost, effects, zone = ActivationZone::Command, limit = limit)
}

const ONCE: ActivationLimit = ActivationLimit::PerTurn(1);
const UNLIMITED: ActivationLimit = ActivationLimit::Unlimited;
const UPKEEP_LIFE: AbilityDef = upkeep(&[Effect::gain_life(1)]);
const UPKEEP_SCRY: AbilityDef = upkeep(&[Effect::scry(1)]);
const DRAW_LIFE: AbilityDef = triggered!(Trigger::Draws(PlayerRel::You), &[Effect::gain_life(1)]);
const ENTER_LIFE: AbilityDef = triggered!(
    Trigger::EntersBattlefield(&f!(your CREATURE)),
    &[Effect::gain_life(1)]
);
const TOKEN_LIFE: AbilityDef = triggered!(
    Trigger::EntersBattlefield(&f!(your token CREATURE)),
    &[Effect::gain_life(1)]
);
const DEATH_LIFE: AbilityDef =
    triggered!(Trigger::Dies(&f!(your CREATURE)), &[Effect::gain_life(1)]);
const LIFE_DRAIN: AbilityDef = triggered!(
    Trigger::GainsLife(PlayerRel::You),
    &[Effect::LoseLife {
        amount: Amount::Fixed(1),
        target: PlayerRel::EachOpponent
    }]
);
const LOOT: AbilityDef = paid(
    cost!("{2}"),
    &[
        Effect::draw(1),
        Effect::DiscardForPlayers {
            who: PlayerRel::You,
            count: 1,
        },
    ],
    ONCE,
);
const PLANT: Effect = Effect::CreateTokensFor {
    player: PlayerRel::You,
    token: &super::TOWER_SEEDLING,
    amount: Amount::Fixed(1),
};
const UPKEEP_PLANT: AbilityDef = upkeep(&[PLANT]);
const TOKEN_GUARD: AbilityDef = static_ability!(f!(your token CREATURE), Modifier::ModifyPT(0, 1));
const TOKEN_GROWTH: AbilityDef = static_ability!(f!(your token CREATURE), Modifier::ModifyPT(1, 1));
const TOKEN_HASTE: AbilityDef = static_ability!(
    f!(your token CREATURE),
    Modifier::AddKeyword(KeywordSet::HASTE)
);
const TOKEN_TRAMPLE: AbilityDef = static_ability!(
    f!(your token CREATURE),
    Modifier::AddKeyword(KeywordSet::TRAMPLE)
);
const TOKEN_VIGILANCE: AbilityDef = static_ability!(
    f!(your token CREATURE),
    Modifier::AddKeyword(KeywordSet::VIGILANCE)
);
const TOKEN_LIFELINK: AbilityDef = static_ability!(
    f!(your token CREATURE),
    Modifier::AddKeyword(KeywordSet::LIFELINK)
);
const NONCREATURE_SPELL: Filter =
    Filter::And(&[Filter::ControlledByYou, Filter::Not(&Filter::CREATURE)]);
const SPELL_LIFE: AbilityDef = triggered!(
    Trigger::SpellCast(&NONCREATURE_SPELL),
    &[Effect::gain_life(1)]
);
const SPELL_PLANT: AbilityDef = triggered!(Trigger::SpellCast(&NONCREATURE_SPELL), &[PLANT]);
const CHARGE_ONE: Effect = Effect::AddCounter {
    kind: CounterKind::Charge,
    amount: Amount::Fixed(1),
};
const UPKEEP_CHARGE: AbilityDef = upkeep(&[CHARGE_ONE]);

/// Exact definition order matches the append-only Tower family/rarity IDs.
pub const RUN_RELICS: [[&TokenDef; 4]; 18] = [
    [
        &TOWER_RUN_HEARTWOOD_COMMON,
        &TOWER_RUN_HEARTWOOD_UNCOMMON,
        &TOWER_RUN_HEARTWOOD_RARE,
        &TOWER_RUN_HEARTWOOD_MYTHIC,
    ],
    [
        &TOWER_RUN_SATCHEL_COMMON,
        &TOWER_RUN_SATCHEL_UNCOMMON,
        &TOWER_RUN_SATCHEL_RARE,
        &TOWER_RUN_SATCHEL_MYTHIC,
    ],
    [
        &TOWER_RUN_COMPASS_COMMON,
        &TOWER_RUN_COMPASS_UNCOMMON,
        &TOWER_RUN_COMPASS_RARE,
        &TOWER_RUN_COMPASS_MYTHIC,
    ],
    [
        &TOWER_RUN_GROVE_COMMON,
        &TOWER_RUN_GROVE_UNCOMMON,
        &TOWER_RUN_GROVE_RARE,
        &TOWER_RUN_GROVE_MYTHIC,
    ],
    [
        &TOWER_RUN_CHALICE_COMMON,
        &TOWER_RUN_CHALICE_UNCOMMON,
        &TOWER_RUN_CHALICE_RARE,
        &TOWER_RUN_CHALICE_MYTHIC,
    ],
    [
        &TOWER_RUN_HEARTH_COMMON,
        &TOWER_RUN_HEARTH_UNCOMMON,
        &TOWER_RUN_HEARTH_RARE,
        &TOWER_RUN_HEARTH_MYTHIC,
    ],
    [
        &TOWER_RUN_AMBER_COMMON,
        &TOWER_RUN_AMBER_UNCOMMON,
        &TOWER_RUN_AMBER_RARE,
        &TOWER_RUN_AMBER_MYTHIC,
    ],
    [
        &TOWER_RUN_COIL_COMMON,
        &TOWER_RUN_COIL_UNCOMMON,
        &TOWER_RUN_COIL_RARE,
        &TOWER_RUN_COIL_MYTHIC,
    ],
    [
        &TOWER_RUN_CRADLE_COMMON,
        &TOWER_RUN_CRADLE_UNCOMMON,
        &TOWER_RUN_CRADLE_RARE,
        &TOWER_RUN_CRADLE_MYTHIC,
    ],
    [
        &TOWER_RUN_BANNER_COMMON,
        &TOWER_RUN_BANNER_UNCOMMON,
        &TOWER_RUN_BANNER_RARE,
        &TOWER_RUN_BANNER_MYTHIC,
    ],
    [
        &TOWER_RUN_HARVEST_COMMON,
        &TOWER_RUN_HARVEST_UNCOMMON,
        &TOWER_RUN_HARVEST_RARE,
        &TOWER_RUN_HARVEST_MYTHIC,
    ],
    [
        &TOWER_RUN_ROOTS_COMMON,
        &TOWER_RUN_ROOTS_UNCOMMON,
        &TOWER_RUN_ROOTS_RARE,
        &TOWER_RUN_ROOTS_MYTHIC,
    ],
    [
        &TOWER_RUN_LANTERN_COMMON,
        &TOWER_RUN_LANTERN_UNCOMMON,
        &TOWER_RUN_LANTERN_RARE,
        &TOWER_RUN_LANTERN_MYTHIC,
    ],
    [
        &TOWER_RUN_LENS_COMMON,
        &TOWER_RUN_LENS_UNCOMMON,
        &TOWER_RUN_LENS_RARE,
        &TOWER_RUN_LENS_MYTHIC,
    ],
    [
        &TOWER_RUN_BELL_COMMON,
        &TOWER_RUN_BELL_UNCOMMON,
        &TOWER_RUN_BELL_RARE,
        &TOWER_RUN_BELL_MYTHIC,
    ],
    [
        &TOWER_RUN_SPINDLE_COMMON,
        &TOWER_RUN_SPINDLE_UNCOMMON,
        &TOWER_RUN_SPINDLE_RARE,
        &TOWER_RUN_SPINDLE_MYTHIC,
    ],
    [
        &TOWER_RUN_AEGIS_COMMON,
        &TOWER_RUN_AEGIS_UNCOMMON,
        &TOWER_RUN_AEGIS_RARE,
        &TOWER_RUN_AEGIS_MYTHIC,
    ],
    [
        &TOWER_RUN_ACORN_COMMON,
        &TOWER_RUN_ACORN_UNCOMMON,
        &TOWER_RUN_ACORN_RARE,
        &TOWER_RUN_ACORN_MYTHIC,
    ],
];

/// Resolve only generation-five identities, never an older artifact source.
#[must_use]
pub fn definition(id: u16) -> Option<&'static TokenDef> {
    let zero = usize::from(id.checked_sub(1)?);
    if zero < RUN_RELICS.len() * 4 {
        Some(RUN_RELICS[zero / 4][zero % 4])
    } else {
        runic::definition(zero / 4 - RUN_RELICS.len(), zero % 4)
    }
}
