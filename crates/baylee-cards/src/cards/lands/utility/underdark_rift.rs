//! Underdark Rift — (no cost) — Land
//! Oracle: {T}: Add {C}.
//! Oracle: {5}, {T}, Exile this land: Roll a d10. Put target artifact, creature, or planeswalker into its owner's library just beneath the top X cards of that library, where X is the result. Activate only as a sorcery.
//! Set: AFC #62 — Forgotten Realms Commander | Scryfall ID: da7b5c5b-fba1-4993-9779-bef96bcb0064 | Oracle ID: 199c7604-6b4b-4cec-b6c2-b6b2918b11c8
// PARTIAL — {T}: Add {C}; the second ability needs a die roll and a library
// placement the DSL cannot express.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::UNDERDARK_RIFT,
    oracle_id = "199c7604-6b4b-4cec-b6c2-b6b2918b11c8",
    scryfall_id = "da7b5c5b-fba1-4993-9779-bef96bcb0064",
    faces = &[face!(name = "Underdark Rift", types = TypeSet::LAND,),],
    coverage = Coverage::Partial(
        "no die roll, and no 'put just beneath the top X cards of its owner's library' effect"
    ),
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        // NOT SUPPORTED: "{5}, {T}, Exile this land: Roll a d10. Put target
        // artifact, creature, or planeswalker into its owner's library just
        // beneath the top X cards of that library, where X is the result."
        // — no `Effect` rolls a die; and the placement has no variant,
        // `Effect::PutTargetOnBottomOfLibrary` being the bottom of the
        // library rather than beneath the top X cards of it.
    ],
);
