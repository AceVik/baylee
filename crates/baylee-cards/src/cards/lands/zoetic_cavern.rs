//! Zoetic Cavern — (no cost) — Land
//! Oracle: {T}: Add {C}.
//! Oracle: Morph {2} (You may cast this card face down as a 2/2 creature for {3}. Turn it face up any time for its morph cost.)
//! Set: MKC #311 — Murders at Karlov Manor Commander | Scryfall ID: 37f10035-bf05-460d-9390-433caa2570f4 | Oracle ID: 3763de30-28e1-4689-a71c-07d2fea3a466
// PARTIAL — {T}: Add {C} is built; Morph {2} is not expressible in the DSL.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::ZOETIC_CAVERN,
    oracle_id = "3763de30-28e1-4689-a71c-07d2fea3a466",
    scryfall_id = "37f10035-bf05-460d-9390-433caa2570f4",
    faces = &[face!(name = "Zoetic Cavern", types = TypeSet::LAND,),],
    coverage = Coverage::Partial(
        "Morph {2}: casting this face down as a 2/2 creature for {3}, turning a face-down permanent face up, and the face-down 2/2 body are not expressible"
    ),
    abilities = &[
        // NOT SUPPORTED: Morph {2} (You may cast this card face down as a
        // 2/2 creature for {3}. Turn it face up any time for its morph
        // cost.) — no `AbilityDef`, alternate cost or face-down state
        // exists in the DSL, and morph is not a keyword bit the engine
        // reads.
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
    ],
);
