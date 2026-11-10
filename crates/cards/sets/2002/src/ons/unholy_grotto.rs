//! Unholy Grotto — (no cost) — Land
//! Oracle: {T}: Add {C}.
//! Oracle: {B}, {T}: Put target Zombie card from your graveyard on top of your library.
//! Set: ONS #327 — Onslaught | Scryfall ID: 52f464a9-586c-4cf3-894b-b407c9f4dcb8 | Oracle ID: c28211c6-a5ee-40c3-bb6a-da3e7e73fd95
// IMPLEMENTED — {T} for {C}; {B}, {T} to put a target Zombie card from your
// graveyard on top of its owner's library (Effect::GraveyardToTop).

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes::creature;

static ZOMBIE_CARD: Filter = Filter::HasSubtype(creature::ZOMBIE);

card!(
    index = index::UNHOLY_GROTTO,
    oracle_id = "c28211c6-a5ee-40c3-bb6a-da3e7e73fd95",
    scryfall_id = "52f464a9-586c-4cf3-894b-b407c9f4dcb8",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(name = "Unholy Grotto", types = TypeSet::LAND,),],
    coverage = Coverage::Implemented,
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        activated!(
            cost!("{B}", TapSelf),
            &[Effect::GraveyardToTop {
                target: TargetSpec::CardInGraveyard(&ZOMBIE_CARD, PlayerRel::You),
            }],
            target = Some(TargetSpec::CardInGraveyard(&ZOMBIE_CARD, PlayerRel::You)),
        ),
    ],
);
