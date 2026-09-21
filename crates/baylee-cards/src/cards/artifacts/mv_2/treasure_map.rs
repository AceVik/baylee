//! Treasure Map // Treasure Cove — {2} — Artifact // Land
//! Oracle: {1}, {T}: Scry 1. Put a landmark counter on this artifact. Then if there are three or more landmark counters on it, remove those counters, transform this artifact, and create three Treasure tokens. (They're artifacts with "{T}, Sacrifice this token: Add one mana of any color.")
//! Oracle: (Transforms from Treasure Map.)
//! Oracle: {T}: Add {C}.
//! Oracle: {T}, Sacrifice a Treasure: Draw a card.
//! Set: LCI #267 — The Lost Caverns of Ixalan | Scryfall ID: a924fe1e-a85e-4e14-88d2-ac55130638ab | Oracle ID: 0b55eac6-a745-4bf4-8926-5ce83bc38d7d
//! Face: Treasure Map — {2} — Artifact
//! Face: Treasure Cove —  — Land
// PARTIAL — the back face is whole ({T}: Add {C}, and {T}, Sacrifice a
// Treasure: Draw a card); the front face's {1}, {T} ability states its
// "Scry 1" and stops there, because the counter it puts on itself has no id
// to be named by and nothing in the DSL takes a counter off or branches on
// a count above zero.

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
        "{1}, {T} can only scry: the landmark counter has no id in \
         baylee_cards_dsl::counters, and no effect removes a counter or \
         branches on a count above zero"
    ),
    abilities = &[
        // "…: Scry 1."
        //
        // NOT SUPPORTED: "Put a landmark counter on this artifact" — the
        // printed word is unnameable here: baylee_cards_dsl::counters
        // assigns QUEST, DEPLETION, MINING and STORAGE, and a card names the
        // constant a word stands for rather than a bare
        // CounterKind::Custom(n).
        // NOT SUPPORTED: "Then if there are three or more landmark counters
        // on it, remove those counters" — counters leave a permanent only as
        // a cost (CostPart::RemoveCounterSelf), and the one counter an
        // effect branches on is zero (Effect::IfNoCountersOnSelf).
        // NOT SUPPORTED: "transform this artifact, and create three Treasure
        // tokens" — both are sayable (Effect::ExileSelfReturnAsFace { face:
        // 1 } and Effect::CreateTokenN over crate::tokens::TREASURE), but
        // they hang off the check the sentence above cannot make.
        activated!(cost!("{1}", TapSelf), &[Effect::scry(1)]),
    ],
);
