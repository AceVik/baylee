//! Compile-time composition of original charged equipment. No runtime interpreter.
use super::{NONCREATURE_SPELL, UPKEEP_CHARGE};
use baylee_cards_dsl::prelude::*;
use baylee_tower_relics::{Sigil, Vessel};

mod generated;
mod rewards;
pub use generated::*;

const fn event(sigil: Sigil) -> Trigger {
    match sigil {
        Sigil::Dawn => Trigger::StepBegin {
            step: StepKind::Upkeep,
            whose: PlayerRel::You,
        },
        Sigil::Dusk => Trigger::StepBegin {
            step: StepKind::End,
            whose: PlayerRel::You,
        },
        Sigil::Root => Trigger::EntersBattlefield(&Filter::YOUR_LAND),
        Sigil::Hearth => Trigger::EntersBattlefield(&f!(your nontoken CREATURE)),
        Sigil::Bloom => Trigger::EntersBattlefield(&f!(your token CREATURE)),
        Sigil::Ash => Trigger::Dies(&Filter::YOUR_CREATURE),
        Sigil::Ember => Trigger::SpellCast(&NONCREATURE_SPELL),
        Sigil::Lore => Trigger::DrawsExceptFirst(PlayerRel::You),
        Sigil::Pulse => Trigger::GainsLife(PlayerRel::You),
        Sigil::Forge => Trigger::EntersBattlefield(&Filter::YOUR_ARTIFACT),
        Sigil::Veil => Trigger::SpellCast(&f!(your ENCHANTMENT)),
        Sigil::Valor => Trigger::OneOrMoreAttack(&Filter::YOUR_CREATURE),
        Sigil::Fang => Trigger::DealsCombatDamageToOpponent(&Filter::YOUR_CREATURE),
        Sigil::Echo => Trigger::Discarded {
            who: PlayerRel::You,
            filter: &Filter::Any,
        },
        Sigil::Wraith => Trigger::Dies(&Filter::OPPONENT_CREATURE),
        Sigil::Star => Trigger::SpellCastOrCopied(&Filter::And(&[
            Filter::ControlledByYou,
            Filter::INSTANT_OR_SORCERY,
        ])),
    }
}

const CHARGES: [&[Effect]; 2] = [
    &[Effect::AddCounter {
        kind: CounterKind::Charge,
        amount: Amount::Fixed(1),
    }],
    &[Effect::AddCounter {
        kind: CounterKind::Charge,
        amount: Amount::Fixed(2),
    }],
];

const fn charge(sigil: Sigil, tier: usize) -> AbilityDef {
    triggered!(event(sigil), CHARGES[if tier >= 2 { 1 } else { 0 }])
}

const PAYMENTS: [CostPart; 7] = [
    CostPart::RemoveCounterSelf {
        kind: CounterKind::Charge,
        n: 1,
    },
    CostPart::RemoveCounterSelf {
        kind: CounterKind::Charge,
        n: 2,
    },
    CostPart::RemoveCounterSelf {
        kind: CounterKind::Charge,
        n: 3,
    },
    CostPart::RemoveCounterSelf {
        kind: CounterKind::Charge,
        n: 4,
    },
    CostPart::RemoveCounterSelf {
        kind: CounterKind::Charge,
        n: 5,
    },
    CostPart::RemoveCounterSelf {
        kind: CounterKind::Charge,
        n: 6,
    },
    CostPart::RemoveCounterSelf {
        kind: CounterKind::Charge,
        n: 7,
    },
];

const fn active_for(sigil: Sigil, vessel: Vessel, tier: usize) -> AbilityDef {
    let Some(level) = (baylee_tower_relics::Recipe { sigil, vessel }).level(tier) else {
        panic!("known rarity")
    };
    let cost = Cost {
        mana: match level.mana {
            0 => ManaCost::ZERO,
            1 => mana!("{1}"),
            2 => mana!("{2}"),
            _ => panic!("known cost"),
        },
        parts: std::slice::from_ref(&PAYMENTS[level.charges as usize - 1]),
    };
    let limit = if level.activations == 0 {
        ActivationLimit::Unlimited
    } else {
        ActivationLimit::PerTurn(level.activations)
    };
    if matches!(vessel, Vessel::Flask) {
        super::mana(cost, rewards::of(vessel, tier), limit)
    } else {
        super::paid(cost, rewards::of(vessel, tier), limit)
    }
}

const DISCARD_LIFE: AbilityDef = triggered!(
    Trigger::Discarded {
        who: PlayerRel::You,
        filter: &Filter::Any
    },
    &[Effect::gain_life(1)]
);

const fn bonus(vessel: Vessel) -> AbilityDef {
    match vessel {
        Vessel::Chalice | Vessel::Tome => super::DRAW_LIFE,
        Vessel::Scepter => triggered!(
            Trigger::OneOrMoreAttack(&Filter::YOUR_CREATURE),
            &[Effect::LoseLife {
                amount: Amount::Fixed(1),
                target: PlayerRel::EachOpponent
            }]
        ),
        Vessel::Prism => super::UPKEEP_SCRY,
        Vessel::Seed => super::TOKEN_VIGILANCE,
        Vessel::Horn => super::TOKEN_GUARD,
        Vessel::Flask => super::SPELL_LIFE,
        Vessel::Bulwark => static_ability!(
            Filter::YOUR_CREATURE,
            Modifier::AddKeyword(KeywordSet::VIGILANCE)
        ),
        Vessel::Banner => super::TOKEN_HASTE,
        Vessel::Lens => DISCARD_LIFE,
    }
}

const fn legend(vessel: Vessel) -> AbilityDef {
    super::upkeep(rewards::of(vessel, 3))
}

/// Resolve a closed family/tier pair into its native emblem definition.
pub fn definition(index: usize, tier: usize) -> Option<&'static TokenDef> {
    RELICS.get(index)?.get(tier).copied()
}

/// Text is materialized by the same catalog generator, including native seats.
pub fn ability_text(token: &TokenDef, index: usize) -> Option<&'static str> {
    // Presentation lookup only: addresses establish compiled identity, never
    // game order or randomness. One packed allocation replaces a 640-row scan.
    static INDEX: std::sync::LazyLock<Box<[(usize, usize, usize)]>> =
        std::sync::LazyLock::new(|| {
            let mut rows: Vec<_> = RELICS
                .iter()
                .enumerate()
                .flat_map(|(recipe, tiers)| {
                    tiers
                        .iter()
                        .enumerate()
                        .map(move |(tier, token)| (std::ptr::from_ref(*token).addr(), recipe, tier))
                })
                .collect();
            rows.sort_unstable_by_key(|row| row.0);
            rows.into_boxed_slice()
        });
    if !token.types.is_empty() {
        return None;
    }
    let found = INDEX
        .binary_search_by_key(&std::ptr::from_ref(token).addr(), |row| row.0)
        .ok()?;
    let (_, recipe, tier) = INDEX[found];
    TEXTS[recipe][tier].get(index).copied()
}
