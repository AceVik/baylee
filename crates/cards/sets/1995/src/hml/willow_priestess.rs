//! Willow Priestess — {2}{G}{G} — Creature — Faerie Druid
//! Oracle: {T}: You may put a Faerie permanent card from your hand onto the battlefield.
//! Oracle: {2}{G}: Target green creature gains protection from black until end of turn.
//! Set: ME3 #138 — Masters Edition III | Scryfall ID: 31479296-5ea7-470c-9d3f-257e67844fbc | Oracle ID: 630eab2b-c1fd-46bb-8ae0-ff837e46db1c
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

const FAERIE_PERMANENT: Filter = Filter::And(&[
    Filter::HasSubtype(subtypes::creature::FAERIE),
    Filter::Or(&[
        Filter::ARTIFACT,
        Filter::CREATURE,
        Filter::ENCHANTMENT,
        Filter::LAND,
        Filter::PLANESWALKER,
    ]),
]);

const GREEN_CREATURE: Filter = Filter::And(&[
    Filter::CREATURE,
    Filter::HasColor(ColorSet::from_slice(&[Color::Green])),
]);

const BLACK: Filter = Filter::HasColor(ColorSet::of(Color::Black));

card!(
    index = index::WILLOW_PRIESTESS,
    oracle_id = "630eab2b-c1fd-46bb-8ae0-ff837e46db1c",
    scryfall_id = "31479296-5ea7-470c-9d3f-257e67844fbc",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Willow Priestess",
        mana_cost = mana!("{2}{G}{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::FAERIE, subtypes::creature::DRUID],
        power = Some(2),
        toughness = Some(2),
    ),],
    abilities = &[
        activated!(
            Cost::TAP,
            &[Effect::PutFromHandOntoBattlefield {
                tapped: false,
                filter: &FAERIE_PERMANENT,
                mana_value: None,
                optional: true,
            }],
        ),
        activated!(
            cost!("{2}{G}"),
            &[Effect::ModifyTargets {
                modifier: Modifier::ProtectionFrom(&BLACK),
                duration: Duration::UntilEndOfTurn,
            }],
            target = Some(TargetSpec::Object(&GREEN_CREATURE)),
        ),
    ],
);
