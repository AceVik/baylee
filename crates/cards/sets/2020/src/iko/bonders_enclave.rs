//! Bonders' Enclave — (no cost) — Land
//! Oracle: {T}: Add {C}.
//! Oracle: {3}, {T}: Draw a card. Activate only if you control a creature with power 4 or greater.
//! Set: OTC #274 — Outlaws of Thunder Junction Commander | Scryfall ID: d174f42c-dcf2-4da9-812b-7e1d0991d466 | Oracle ID: f33ce38a-34ec-4b65-a0fc-160484a02007
// IMPLEMENTED — {T}: Add {C}, and the {3}, {T} draw behind the power gate
// `Filter::PowerAtLeast` now states.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::BONDERS_ENCLAVE,
    oracle_id = "f33ce38a-34ec-4b65-a0fc-160484a02007",
    scryfall_id = "d174f42c-dcf2-4da9-812b-7e1d0991d466",
    faces = &[face!(name = "Bonders' Enclave", types = TypeSet::LAND,)],
    coverage = Coverage::Implemented,
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        activated!(
            cost!("{3}", TapSelf),
            &[Effect::draw(1)],
            // The gate, as the card prints it: a creature **you control**,
            // its power read after the layers.
            condition = Some(Condition::ControlCount(
                &Filter::YOUR_CREATURE_WITH_POWER_4_OR_GREATER,
                1
            )),
        ),
    ],
);
