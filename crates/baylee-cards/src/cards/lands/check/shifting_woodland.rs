//! Shifting Woodland — (no cost) — Land
//! Oracle: This land enters tapped unless you control a Forest.
//! Oracle: {T}: Add {G}.
//! Oracle: Delirium — {2}{G}{G}: This land becomes a copy of target permanent card in your graveyard until end of turn. Activate only if there are four or more card types among cards in your graveyard.
//! Set: MH3 #228 — Modern Horizons 3 | Scryfall ID: 059164e1-894d-4586-9800-e60d6fbd6eb6 | Oracle ID: 7c2a4fe5-43e8-4e20-bef2-0278d18afc4b
// PARTIAL — the enter clause (TappedUnless a Forest you control) and the mana
// ability are built; the delirium copy ability is dropped (see the
// NOT SUPPORTED line and `coverage`).

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes::land;

/// "…unless you control a Forest."
static FOREST_YOU_CONTROL: Filter =
    Filter::And(&[Filter::HasSubtype(land::FOREST), Filter::ControlledByYou]);

// NOT SUPPORTED: "Delirium — {2}{G}{G}: This land becomes a copy of target
// permanent card in your graveyard until end of turn. Activate only if there
// are four or more card types among cards in your graveyard." — the whole
// ability is off the card. No `Condition` counts card types in *your*
// graveyard (`OpponentGraveyardCountAtLeast` counts an opponent's cards), and
// no effect turns the source into a copy of a graveyard card: the two copy
// variants are enter-time choices and `Modifier::BecomeCopyOf` needs an
// `ObjectId` no `static` can hold.

card!(
    index = index::SHIFTING_WOODLAND,
    oracle_id = "7c2a4fe5-43e8-4e20-bef2-0278d18afc4b",
    scryfall_id = "059164e1-894d-4586-9800-e60d6fbd6eb6",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Shifting Woodland",
        types = TypeSet::LAND,
        enter_modifiers = &[EnterModifier::TappedUnless(&FOREST_YOU_CONTROL)],
    ),],
    coverage = Coverage::Partial(
        "Delirium — {2}{G}{G}: no Condition for \"four or more card types among cards in your \
         graveyard\" (OpponentGraveyardCountAtLeast counts an opponent's cards, not your card \
         types), and no effect makes the source become a copy of a targeted graveyard card \
         (CopyOnEnter/CopyOnEnterUntilEot fire as it enters; Modifier::BecomeCopyOf takes an \
         ObjectId a static cannot hold)."
    ),
    abilities = &[mana_ability!(&[Effect::mana(ManaColor::Green, 1)])],
);
