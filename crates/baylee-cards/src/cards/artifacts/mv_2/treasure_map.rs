//! Treasure Map // Treasure Cove — {2} — Artifact // Land
//! Oracle: {1}, {T}: Scry 1. Put a landmark counter on this artifact. Then if there are three or more landmark counters on it, remove those counters, transform this artifact, and create three Treasure tokens. (They're artifacts with "{T}, Sacrifice this token: Add one mana of any color.")
//! Oracle: (Transforms from Treasure Map.)
//! Oracle: {T}: Add {C}.
//! Oracle: {T}, Sacrifice a Treasure: Draw a card.
//! Set: LCI #267 — The Lost Caverns of Ixalan | Scryfall ID: a924fe1e-a85e-4e14-88d2-ac55130638ab | Oracle ID: 0b55eac6-a745-4bf4-8926-5ce83bc38d7d
//! Face: Treasure Map — {2} — Artifact
//! Face: Treasure Cove —  — Land
// PARTIAL — the back face is whole ({T}: Add {C}, and {T}, Sacrifice a
// Treasure: Draw a card); the front face's {1}, {T} ability scries and puts
// its landmark counter, and stops before the three-counter clause, because
// nothing in the DSL takes a counter off as an effect and nothing transforms
// a permanent in place (#206).

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes::artifact;

/// Treasure Cove's two printed abilities.
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
            // A back face reached only by transforming and printing no mana
            // cost; left at the default it would be offered as a mode of the
            // front spell for nothing.
            castable_from_hand = false,
        ),
    ],
    coverage = Coverage::Partial(
        "{1}, {T} scries and puts a landmark counter, and stops there: the \
         three-counter clause needs an effect that removes counters and a \
         transform in place (#206)"
    ),
    abilities = &[
        // "…: Scry 1. Put a landmark counter on this artifact." Untargeted,
        // so the counter goes on the map itself.
        //
        // NOT SUPPORTED: "Then if there are three or more landmark counters
        // on it, remove those counters" — the check is sayable
        // (Effect::IfCondition over Condition::CountersOnSelf), but counters
        // leave a permanent only as a cost (CostPart::RemoveCounterSelf).
        // NOT SUPPORTED: "transform this artifact, and create three Treasure
        // tokens" — the Treasures are sayable (Effect::CreateTokenN over
        // crate::tokens::TREASURE); the transform is not (#206), and the
        // exile-and-return Thaumatic Compass writes instead
        // (Effect::ExileSelfReturnAsFace) would bring Treasure Cove back
        // untapped, as a new object, on the turn the map was tapped for it.
        // Written without the transform, every activation from the third on
        // would make three.
        activated!(
            cost!("{1}", TapSelf),
            &[
                Effect::scry(1),
                Effect::AddCounter {
                    kind: counters::LANDMARK,
                    amount: Amount::Fixed(1),
                },
            ]
        ),
    ],
);
