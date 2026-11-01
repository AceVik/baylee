//! Burning Palm Efreet — {2}{R}{R} — Creature — Efreet
//! Oracle: {1}{R}{R}: This creature deals 2 damage to target creature with flying and that creature loses flying until end of turn.
//! Set: MIR #161 — Mirage | Scryfall ID: a194488a-1d6f-4bc3-8f30-82e2c3c91389 | Oracle ID: 55cbc0e8-eeec-433a-b70c-3501238b658b
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

const CREATURE_WITH_FLYING: Filter =
    Filter::And(&[Filter::CREATURE, Filter::HasKeyword(KeywordSet::FLYING)]);

card!(
    index = index::BURNING_PALM_EFREET,
    oracle_id = "55cbc0e8-eeec-433a-b70c-3501238b658b",
    scryfall_id = "a194488a-1d6f-4bc3-8f30-82e2c3c91389",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Burning Palm Efreet",
        mana_cost = mana!("{2}{R}{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::EFREET],
        power = Some(2),
        toughness = Some(2),
    ),],
    abilities = &[activated!(
        cost!("{1}{R}{R}"),
        &[
            Effect::DealDamage {
                target: TargetSpec::Object(&CREATURE_WITH_FLYING),
                amount: Amount::Fixed(2),
            },
            Effect::continuous(
                &Filter::This,
                Modifier::RemoveKeyword(KeywordSet::FLYING),
                Duration::UntilEndOfTurn,
            ),
        ],
        target = Some(TargetSpec::Object(&CREATURE_WITH_FLYING)),
    )],
);
