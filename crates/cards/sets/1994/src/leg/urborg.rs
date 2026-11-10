//! Urborg — (no cost) — Legendary Land
//! Oracle: {T}: Add {B}.
//! Oracle: {T}: Target creature loses first strike or swampwalk until end of turn.
//! Set: ME3 #214 — Masters Edition III | Scryfall ID: 5319f782-2713-43b1-9b28-0d1ec7a39203 | Oracle ID: b6114962-035e-4e7f-9009-4739bf83a05a

// IMPLEMENTED — all printed abilities; exercised in engine card tests.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::URBORG,
    oracle_id = "b6114962-035e-4e7f-9009-4739bf83a05a",
    scryfall_id = "5319f782-2713-43b1-9b28-0d1ec7a39203",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Urborg",
        types = TypeSet::LAND,
        supertypes = SupertypeSet::LEGENDARY,
    ),],
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Black, 1)]),
        activated!(
            Cost::TAP,
            &[Effect::ChooseOne {
                choices: &[
                    Effect::ModifyTargets {
                        modifier: Modifier::RemoveKeyword(KeywordSet::FIRST_STRIKE),
                        duration: Duration::UntilEndOfTurn
                    },
                    Effect::ModifyTargets {
                        modifier: Modifier::RemoveKeyword(KeywordSet::SWAMPWALK),
                        duration: Duration::UntilEndOfTurn
                    },
                ]
            }],
            target = Some(TargetSpec::Object(&Filter::CREATURE))
        ),
    ],
);
