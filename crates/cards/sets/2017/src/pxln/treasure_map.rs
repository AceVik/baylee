//! Treasure Map // Treasure Cove — {2} — Artifact // Land
//! Oracle: {1}, {T}: Scry 1. Put a landmark counter on this artifact. Then if there are three or more landmark counters on it, remove those counters, transform this artifact, and create three Treasure tokens. (They're artifacts with "{T}, Sacrifice this token: Add one mana of any color.")
//! Oracle: (Transforms from Treasure Map.)
//! Oracle: {T}: Add {C}.
//! Oracle: {T}, Sacrifice a Treasure: Draw a card.
//! Set: LCI #267 — The Lost Caverns of Ixalan | Scryfall ID: a924fe1e-a85e-4e14-88d2-ac55130638ab | Oracle ID: 0b55eac6-a745-4bf4-8926-5ce83bc38d7d
//! Face: Treasure Map — {2} — Artifact
//! Face: Treasure Cove —  — Land

// IMPLEMENTED — all printed abilities; exercised in engine card tests.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes::artifact;

static BACK_ABILITIES: &[AbilityDef] = &[
    mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
    activated!(
        cost!(TapSelf, Sacrifice(&Filter::HasSubtype(artifact::TREASURE))),
        &[Effect::draw(1)]
    ),
];

card!(
    index = index::TREASURE_MAP,
    oracle_id = "0b55eac6-a745-4bf4-8926-5ce83bc38d7d",
    scryfall_id = "a924fe1e-a85e-4e14-88d2-ac55130638ab",
    faces = &[
        face!(
            name = "Treasure Map",
            mana_cost = mana!("{2}"),
            types = TypeSet::ARTIFACT,
        ),
        face!(
            name = "Treasure Cove",
            types = TypeSet::LAND,
            abilities = BACK_ABILITIES,
            castable_from_hand = false,
        ),
    ],
    coverage = Coverage::Implemented,
    abilities = &[activated!(
        cost!("{1}", TapSelf),
        &[
            Effect::scry(1),
            Effect::AddCounter {
                kind: counters::LANDMARK,
                amount: Amount::Fixed(1),
            },
            Effect::IfCondition {
                condition: Condition::CountersOnSelf(counters::LANDMARK, 3),
                then: &[
                    // Removal is capped by the source's u16 counter pool.
                    Effect::RemoveCounterSelf {
                        kind: counters::LANDMARK,
                        n: u16::MAX
                    },
                    Effect::TransformSource,
                    Effect::CreateTokenN {
                        token: &crate::tokens::TREASURE,
                        amount: Amount::Fixed(3)
                    },
                ],
                otherwise: &[],
            },
        ]
    ),],
);
