//! Goblin Wizard — {2}{R}{R} — Creature — Goblin Wizard
//! Oracle: {T}: You may put a Goblin permanent card from your hand onto the battlefield.
//! Oracle: {R}: Target Goblin gains protection from white until end of turn.
//! Set: ME1 #97 — Masters Edition | Scryfall ID: 09167603-93ab-488b-88eb-37f7cf9e3e19 | Oracle ID: 912c99a0-ee97-4bb9-8ab7-2e469d74fa03

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

const GOBLIN_PERMANENT: Filter = Filter::And(&[
    Filter::HasSubtype(subtypes::creature::GOBLIN),
    Filter::Or(&[
        Filter::ARTIFACT,
        Filter::CREATURE,
        Filter::ENCHANTMENT,
        Filter::LAND,
        Filter::PLANESWALKER,
    ]),
]);

const GOBLIN: Filter = Filter::HasSubtype(subtypes::creature::GOBLIN);
const WHITE: Filter = Filter::HasColor(ColorSet::of(Color::White));

card!(
    index = index::GOBLIN_WIZARD,
    oracle_id = "912c99a0-ee97-4bb9-8ab7-2e469d74fa03",
    scryfall_id = "09167603-93ab-488b-88eb-37f7cf9e3e19",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Goblin Wizard",
        mana_cost = mana!("{2}{R}{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::GOBLIN, subtypes::creature::WIZARD],
        power = Some(1),
        toughness = Some(1),
    ),],
    abilities = &[
        activated!(
            Cost::TAP,
            &[Effect::PutFromHandOntoBattlefield {
                tapped: false,
                filter: &GOBLIN_PERMANENT,
                mana_value: None,
                optional: true,
            }],
        ),
        activated!(
            cost!("{R}"),
            &[Effect::ModifyTargets {
                modifier: Modifier::ProtectionFrom(&WHITE),
                duration: Duration::UntilEndOfTurn,
            }],
            target = Some(TargetSpec::Object(&GOBLIN)),
        ),
    ],
);
