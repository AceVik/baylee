//! Thousand Moons Smithy // Barracks of the Thousand — {2}{W}{W} — Legendary Artifact // Legendary Artifact Land
//! Oracle: When Thousand Moons Smithy enters, create a white Gnome Soldier artifact creature token with "This token's power and toughness are each equal to the number of artifacts and/or creatures you control."
//! Oracle: At the beginning of your first main phase, you may tap five untapped artifacts and/or creatures you control. If you do, transform Thousand Moons Smithy.
//! Oracle: (Transforms from Thousand Moons Smithy.)
//! Oracle: {T}: Add {W}.
//! Oracle: Whenever you cast an artifact or creature spell using mana produced by Barracks of the Thousand, create a white Gnome Soldier artifact creature token with "This token's power and toughness are each equal to the number of artifacts and/or creatures you control."
//! Set: LCI #39 — The Lost Caverns of Ixalan | Scryfall ID: 4a6bec46-1acd-4726-b8d9-3045ac6a2ea2 | Oracle ID: 32af5f7b-a970-484a-9aff-226749551d32
//! Face: Thousand Moons Smithy — {2}{W}{W} — Legendary Artifact
//! Face: Barracks of the Thousand —  — Legendary Artifact Land

use crate::generated_tokens;
use baylee_cards_dsl::prelude::*;
static ARTIFACTS_OR_CREATURES: Filter =
    Filter::And(&[Filter::ARTIFACT_OR_CREATURE, Filter::ControlledByYou]);
static BACK_MANA: &[AbilityDef] = &[
    mana_ability!(&[Effect::mana(ManaColor::White, 1)]),
    triggered!(
        Trigger::SpellCastUsingSourceMana(&Filter::And(&[
            Filter::ARTIFACT_OR_CREATURE,
            Filter::ControlledByYou
        ])),
        &[Effect::CreateToken {
            token: &generated_tokens::GNOME_SOLDIER_WHITE_DYNAMIC
        }]
    ),
];

card!(
    index = index::THOUSAND_MOONS_SMITHY,
    oracle_id = "32af5f7b-a970-484a-9aff-226749551d32",
    scryfall_id = "4a6bec46-1acd-4726-b8d9-3045ac6a2ea2",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[
        face!(
            name = "Thousand Moons Smithy",
            mana_cost = mana!("{2}{W}{W}"),
            types = TypeSet::ARTIFACT,
            supertypes = SupertypeSet::LEGENDARY,
        ),
        face!(
            name = "Barracks of the Thousand",
            castable_from_hand = false,
            types = TypeSet::ARTIFACT.union(TypeSet::LAND),
            supertypes = SupertypeSet::LEGENDARY,
            abilities = BACK_MANA,
        ),
    ],
    coverage = Coverage::Implemented,
    abilities = &[
        triggered!(
            Trigger::ETB,
            &[Effect::CreateToken {
                token: &generated_tokens::GNOME_SOLDIER_WHITE_DYNAMIC
            }]
        ),
        triggered!(
            Trigger::StepBegin {
                step: StepKind::FirstMain,
                whose: PlayerRel::You
            },
            &[Effect::PlayerMayPayCostThen {
                player: PlayerRel::You,
                cost: &CostPart::TapN {
                    filter: &ARTIFACTS_OR_CREATURES,
                    count: 5
                },
                effect: &Effect::TransformSource
            }]
        ),
    ],
);
