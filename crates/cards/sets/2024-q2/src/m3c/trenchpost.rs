//! Trenchpost — (no cost) — Land — Locus
//! Oracle: {T}: Add {C}.
//! Oracle: {3}, {T}: Target player mills a card for each Locus you control.
//! Set: M3C #83 — Modern Horizons 3 Commander | Scryfall ID: 4afcabf8-8f84-489d-8496-5bec55b351bd | Oracle ID: 42f1ccb8-eda0-4828-ac07-82d4e950d7e1
// IMPLEMENTED — {T}: Add {C}; {3}, {T}: a target player mills one card for
// each Locus you control.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static LOCUS_YOU_CONTROL: Filter = Filter::And(&[
    Filter::HasSubtype(subtypes::land::LOCUS),
    Filter::ControlledByYou,
]);

card!(
    index = index::TRENCHPOST,
    oracle_id = "42f1ccb8-eda0-4828-ac07-82d4e950d7e1",
    scryfall_id = "4afcabf8-8f84-489d-8496-5bec55b351bd",
    faces = &[face!(
        name = "Trenchpost",
        types = TypeSet::LAND,
        subtypes = &[subtypes::land::LOCUS],
    ),],
    coverage = Coverage::Implemented,
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        activated!(
            cost!("{3}", TapSelf),
            &[Effect::Mill {
                amount: Amount::CountOf {
                    filter: &LOCUS_YOU_CONTROL,
                    zone: ZoneSel::Battlefield,
                },
                target: PlayerRel::Chosen,
            }],
            target = Some(TargetSpec::AnyPlayer)
        ),
    ],
);
