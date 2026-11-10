//! Goblin Snowman — {3}{R} — Creature — Goblin
//! Oracle: Whenever this creature blocks, prevent all combat damage that would be dealt to and dealt by it this turn.
//! Oracle: {T}: This creature deals 1 damage to target creature it's blocking.
//! Set: TSB #64 — Time Spiral Timeshifted | Scryfall ID: 24dc6db4-671e-4d7d-ba11-89194ce0b2ad | Oracle ID: 38e9ad30-4bbf-4b58-8bb2-47520ce351a3
use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static BLOCKED_BY_SOURCE: Filter = Filter::And(&[Filter::CREATURE, Filter::BlockedBySource]);

card!(
    index = index::GOBLIN_SNOWMAN,
    oracle_id = "38e9ad30-4bbf-4b58-8bb2-47520ce351a3",
    scryfall_id = "24dc6db4-671e-4d7d-ba11-89194ce0b2ad",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Goblin Snowman",
        mana_cost = mana!("{3}{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::GOBLIN],
        power = Some(1),
        toughness = Some(1),
    ),],
    abilities = &[
        triggered!(
            Trigger::Blocks(&Filter::This),
            &[
                Effect::ModifySource {
                    modifier: Modifier::PreventDamageToIt,
                    duration: Duration::UntilEndOfTurn,
                },
                Effect::ModifySource {
                    modifier: Modifier::PreventDamageFromIt,
                    duration: Duration::UntilEndOfTurn,
                },
            ]
        ),
        activated!(
            cost!(TapSelf),
            &[Effect::DealDamage {
                target: TargetSpec::Object(&BLOCKED_BY_SOURCE),
                amount: Amount::Fixed(1),
            }],
            target = Some(TargetSpec::Object(&BLOCKED_BY_SOURCE))
        ),
    ],
);
