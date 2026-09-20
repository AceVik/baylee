//! Gond Gate — (no cost) — Land — Gate
//! Oracle: Gates you control enter untapped.
//! Oracle: {T}: Add {C}.
//! Oracle: {T}: Add one mana of any color that a Gate you control could produce.
//! Set: CLB #353 — Commander Legends: Battle for Baldur's Gate | Scryfall ID: 746672d9-7c6b-415e-9f34-3cc3ac557008 | Oracle ID: 4306938b-c0db-4e63-a4fb-61628e5ff41f
// PARTIAL — {T}: Add {C} is built. The Gate entry permission and the
// Gate-narrowed any-color mana are not sayable in the DSL and are left off.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::GOND_GATE,
    oracle_id = "4306938b-c0db-4e63-a4fb-61628e5ff41f",
    scryfall_id = "746672d9-7c6b-415e-9f34-3cc3ac557008",
    faces = &[face!(
        name = "Gond Gate",
        types = TypeSet::LAND,
        subtypes = &[subtypes::land::GATE],
    ),],
    coverage = Coverage::Partial(
        "no Modifier grants another object an as-it-enters modifier, and \
         ManaSource::LandColor carries no filter to narrow it to a Gate"
    ),
    abilities = &[
        // NOT SUPPORTED: "Gates you control enter untapped." — every
        // as-it-enters modifier is a field of this face (`FaceDef::enter_modifiers`)
        // and no `Modifier` variant hands one to another permanent.
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        // NOT SUPPORTED: "{T}: Add one mana of any color that a Gate you
        // control could produce." — `ManaSource::LandColor` reads every land
        // its controller has and takes only `mine`; nothing narrows it to the
        // Gate subtype, so the printed sentence has no spelling here.
    ],
);
