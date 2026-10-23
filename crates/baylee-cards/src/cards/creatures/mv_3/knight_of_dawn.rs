//! Knight of Dawn — {1}{W}{W} — Creature — Human Knight
//! Oracle: First strike
//! Oracle: {W}{W}: This creature gains protection from the color of your choice until end of turn.
//! Set: TMP #26 — Tempest | Scryfall ID: bf0e5034-a134-4eb6-af8e-b2419b92b3a6 | Oracle ID: 51262155-f1be-45a8-a21f-94e920a93e55
// IMPLEMENTED — first strike and a non-targeted choice of protection color until end of turn.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::KNIGHT_OF_DAWN,
    oracle_id = "51262155-f1be-45a8-a21f-94e920a93e55",
    scryfall_id = "bf0e5034-a134-4eb6-af8e-b2419b92b3a6",
    color_identity = ColorSet::from_slice(&[Color::White]),
    keywords = KeywordSet::FIRST_STRIKE,
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Knight of Dawn",
        mana_cost = mana!("{1}{W}{W}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::KNIGHT],
        power = Some(2),
        toughness = Some(2),
    ),],
    abilities = &[activated!(
        cost!("{W}{W}"),
        &[Effect::ChooseOne {
            choices: &[
                Effect::ModifySource {
                    modifier: Modifier::ProtectionFrom(&Filter::HasColor(ColorSet::from_slice(&[
                        Color::White
                    ]))),
                    duration: Duration::UntilEndOfTurn
                },
                Effect::ModifySource {
                    modifier: Modifier::ProtectionFrom(&Filter::HasColor(ColorSet::from_slice(&[
                        Color::Blue
                    ]))),
                    duration: Duration::UntilEndOfTurn
                },
                Effect::ModifySource {
                    modifier: Modifier::ProtectionFrom(&Filter::HasColor(ColorSet::from_slice(&[
                        Color::Black
                    ]))),
                    duration: Duration::UntilEndOfTurn
                },
                Effect::ModifySource {
                    modifier: Modifier::ProtectionFrom(&Filter::HasColor(ColorSet::from_slice(&[
                        Color::Red
                    ]))),
                    duration: Duration::UntilEndOfTurn
                },
                Effect::ModifySource {
                    modifier: Modifier::ProtectionFrom(&Filter::HasColor(ColorSet::from_slice(&[
                        Color::Green
                    ]))),
                    duration: Duration::UntilEndOfTurn
                }
            ]
        }],
    )],
);
