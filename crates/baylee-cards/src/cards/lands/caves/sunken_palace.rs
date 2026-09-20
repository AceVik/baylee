//! Sunken Palace — (no cost) — Land — Cave
//! Oracle: This land enters tapped.
//! Oracle: {T}: Add {U}.
//! Oracle: {1}{U}, {T}, Exile seven cards from your graveyard: Add {U}. When you spend this mana to cast a spell or activate an ability, copy that spell or ability. You may choose new targets for the copy. (Mana abilities can't be copied.)
//! Set: M3C #81 — Modern Horizons 3 Commander | Scryfall ID: e44ee47c-95de-4090-97b6-188585d86b0c | Oracle ID: c098c507-5154-423a-a70b-f6dfd4959cf6
// PARTIAL — the printed entry condition (`EnterModifier::Tapped`) and the
// `{T}: Add {U}` mana ability are built.
// NOT SUPPORTED: "{1}{U}, {T}, Exile seven cards from your graveyard: Add {U}.
// When you spend this mana to cast a spell or activate an ability, copy that
// spell or ability." — `CostPart` has no part that exiles cards from a
// graveyard (`ExileFromHand` is the hand, and nothing carries a count), and
// `SpendRider` has no rider that copies anything (`None`, `Uncounterable`,
// `Scry` only). The whole ability comes off the card rather than being paid
// wrong.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::SUNKEN_PALACE,
    oracle_id = "c098c507-5154-423a-a70b-f6dfd4959cf6",
    scryfall_id = "e44ee47c-95de-4090-97b6-188585d86b0c",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Partial(
        "the {1}{U} ability: no cost part exiles cards from a graveyard, and no spend rider copies the spell or ability the mana is spent on",
    ),
    faces = &[face!(
        name = "Sunken Palace",
        types = TypeSet::LAND,
        subtypes = &[subtypes::land::CAVE],
        enter_modifiers = &[EnterModifier::Tapped],
    ),],
    abilities = &[mana_ability!(&[Effect::mana(ManaColor::Blue, 1)])],
);
