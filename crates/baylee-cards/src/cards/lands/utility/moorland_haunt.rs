//! Moorland Haunt — (no cost) — Land
//! Oracle: {T}: Add {C}.
//! Oracle: {W}{U}, {T}, Exile a creature card from your graveyard: Create a 1/1 white Spirit creature token with flying.
//! Set: VOC #175 — Crimson Vow Commander | Scryfall ID: 74deeeaa-95e9-42c9-89e3-e317ea63e216 | Oracle ID: 5324192b-6687-41e4-8e56-326b21a5dbf3
// PARTIAL — the {T}: Add {C} mana ability; the token ability needs a cost the DSL has not got.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::MOORLAND_HAUNT,
    oracle_id = "5324192b-6687-41e4-8e56-326b21a5dbf3",
    scryfall_id = "74deeeaa-95e9-42c9-89e3-e317ea63e216",
    color_identity = ColorSet::from_slice(&[Color::Blue, Color::White]),
    faces = &[face!(name = "Moorland Haunt", types = TypeSet::LAND,),],
    coverage = Coverage::Partial(
        "the second ability's cost — exile a creature card from your graveyard — has no CostPart, so the ability is dropped rather than offered with its exile skipped"
    ),
    abilities = &[mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)])],
);

// NOT SUPPORTED: {W}{U}, {T}, Exile a creature card from your graveyard: Create a 1/1 white Spirit creature token with flying — no `CostPart` exiles a card from a graveyard (`ExileFromHand` is the hand, `Sacrifice` a battlefield permanent, `ExileSelf` the source), and writing the exile as an effect would offer the activation on an empty graveyard. The token itself is a second gap: no 1/1 white Spirit with flying stands in `crate::tokens`.
