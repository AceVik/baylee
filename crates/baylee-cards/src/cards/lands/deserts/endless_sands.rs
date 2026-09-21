//! Endless Sands — (no cost) — Land — Desert
//! Oracle: {T}: Add {C}.
//! Oracle: {2}, {T}: Exile target creature you control.
//! Oracle: {4}, {T}, Sacrifice this land: Return each creature card exiled with this land to the battlefield under its owner's control.
//! Set: C20 #272 — Commander 2020 | Scryfall ID: f436df63-41b9-416d-8e41-035261d4d0cc | Oracle ID: c4033d97-769f-4811-8b11-f85b8817b7a2
// IMPLEMENTED — {T} for {C}, {2}, {T} exiles target creature you control with a link to this land, and {4}, {T}, sac returns all linked cards to the battlefield under owner's control.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::ENDLESS_SANDS,
    oracle_id = "c4033d97-769f-4811-8b11-f85b8817b7a2",
    scryfall_id = "f436df63-41b9-416d-8e41-035261d4d0cc",
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Endless Sands",
        types = TypeSet::LAND,
        subtypes = &[subtypes::land::DESERT],
    ),],
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        activated!(
            cost!("{2}", TapSelf),
            &[Effect::ExileLinked {
                target: TargetSpec::Object(&Filter::YOUR_CREATURE),
            }],
            target = Some(TargetSpec::Object(&Filter::YOUR_CREATURE)),
        ),
        activated!(
            cost!("{4}", TapSelf, SacrificeSelf),
            &[Effect::ReturnLinkedToBattlefield],
        ),
    ],
);
