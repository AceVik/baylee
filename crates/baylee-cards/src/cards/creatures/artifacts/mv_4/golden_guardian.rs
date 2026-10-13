//! Golden Guardian // Gold-Forge Garrison — {4} — Artifact Creature — Golem // Land
//! Oracle: Defender
//! Oracle: {2}: This creature fights another target creature you control. When this creature dies this turn, return it to the battlefield transformed under your control.
//! Oracle: (Transforms from Golden Guardian.)
//! Oracle: {T}: Add two mana of any one color.
//! Oracle: {4}, {T}: Create a 4/4 colorless Golem artifact creature token.
//! Set: RIX #179 — Rivals of Ixalan | Scryfall ID: 397ba02d-f347-46f7-b028-dd4ba55faa2f | Oracle ID: 58afb897-4d57-4b53-a5c3-b532cb3d5180
//! Face: Golden Guardian — {4} — Artifact Creature — Golem
//! Face: Gold-Forge Garrison —  — Land
// IMPLEMENTED — Defender, {2}: fight another target creature you control and
// return this transformed if it dies this turn, and on Gold-Forge Garrison
// {T}: Add two mana of any one color and {4}, {T}: a 4/4 Golem token.

use crate::generated_tokens;
use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

/// Gold-Forge Garrison — "{T}: Add two mana of any one color." and "{4}, {T}:
/// Create a 4/4 colorless Golem artifact creature token."
///
/// One colour pick for both mana (`mana_choice_dynamic`, not
/// `mana_combination`: "any one color" is a single choice, and the
/// combination spelling would prompt twice).
static BACK_ABILITIES: &[AbilityDef] = &[
    mana_ability!(
        Cost::TAP,
        &[Effect::mana_choice_dynamic(
            ALL_MANA_COLORS,
            Amount::Fixed(2)
        )]
    ),
    activated!(
        cost!("{4}", TapSelf),
        &[Effect::CreateToken {
            token: &generated_tokens::GOLEM_ARTIFACT_4_4
        }]
    ),
];

card!(
    index = index::GOLDEN_GUARDIAN,
    oracle_id = "58afb897-4d57-4b53-a5c3-b532cb3d5180",
    scryfall_id = "397ba02d-f347-46f7-b028-dd4ba55faa2f",
    faces = &[
        face!(
            name = "Golden Guardian",
            mana_cost = mana!("{4}"),
            types = TypeSet::ARTIFACT.union(TypeSet::CREATURE),
            subtypes = &[subtypes::creature::GOLEM],
            power = Some(4),
            toughness = Some(4),
        ),
        face!(
            name = "Gold-Forge Garrison",
            // CR 712.8c: a nonmodal double-faced card is cast as its front
            // face and reaches this one only by transforming.
            castable_from_hand = false,
            types = TypeSet::LAND,
            abilities = BACK_ABILITIES,
        ),
    ],
    coverage = Coverage::Implemented,
    // "This creature fights another target creature you control. When this
    // creature dies this turn, return it to the battlefield transformed under
    // your control."
    abilities = &[activated!(
        cost!("{2}"),
        &[
            Effect::Fight {
                fighter: TargetSlot::This,
                foe: TargetSlot::First,
            },
            Effect::WhenSourceDiesThisTurn {
                effects: &[Effect::ReturnSourceFromGraveyard {
                    face: Some(1),
                    tapped: false,
                    owner_control: false,
                    counters: None,
                }],
            },
        ],
        target = Some(TargetSpec::Object(&Filter::ANOTHER_CREATURE_YOU_CONTROL)),
    )],
    keywords = KeywordSet::DEFENDER,
);
