//! Sawback Manticore — {3}{R}{G} — Creature — Manticore
//! Oracle: {4}: This creature gains flying until end of turn.
//! Oracle: {1}: This creature deals 2 damage to target attacking or blocking creature. Activate only if this creature is attacking or blocking and only once each turn.
//! Set: MIR #281 — Mirage | Scryfall ID: c24e74e9-ce88-48af-a113-b4fe76f963d4 | Oracle ID: 9360d30c-4556-4b41-bd50-08b4367e5efb

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static ATTACKING_OR_BLOCKING: Filter = Filter::Or(&[Filter::Attacking, Filter::Blocking]);
static ATTACKING_OR_BLOCKING_CREATURE: Filter =
    Filter::And(&[Filter::CREATURE, ATTACKING_OR_BLOCKING]);

card!(
    index = index::SAWBACK_MANTICORE,
    oracle_id = "9360d30c-4556-4b41-bd50-08b4367e5efb",
    scryfall_id = "c24e74e9-ce88-48af-a113-b4fe76f963d4",
    color_identity = ColorSet::from_slice(&[Color::Green, Color::Red]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Sawback Manticore",
        mana_cost = mana!("{3}{R}{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::MANTICORE],
        power = Some(2),
        toughness = Some(4),
    ),],
    abilities = &[
        activated!(
            cost!("{4}"),
            &[Effect::continuous(
                &Filter::This,
                Modifier::AddKeyword(KeywordSet::FLYING),
                Duration::UntilEndOfTurn,
            )],
        ),
        activated!(
            cost!("{1}"),
            &[Effect::DealDamage {
                amount: Amount::Fixed(2),
                target: TargetSpec::Object(&ATTACKING_OR_BLOCKING_CREATURE),
            }],
            target = Some(TargetSpec::Object(&ATTACKING_OR_BLOCKING_CREATURE)),
            condition = Some(Condition::SourceMatches(&ATTACKING_OR_BLOCKING)),
            limit = ActivationLimit::PerTurn(1),
        ),
    ],
);
