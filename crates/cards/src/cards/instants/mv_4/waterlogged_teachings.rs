//! Waterlogged Teachings // Inundated Archive — {3}{U/B} — Instant // Land
//! Oracle: Search your library for an instant card or a card with flash, reveal it, put it into your hand, then shuffle.
//! Oracle: This land enters tapped.
//! Oracle: {T}: Add {U} or {B}.
//! Set: MH3 #261 — Modern Horizons 3 | Scryfall ID: 060f9675-4921-4cbb-bae2-54c85c679fd4 | Oracle ID: e6ad1be9-f13d-4590-b3db-e2d0fff46f03
//! Face: Waterlogged Teachings — {3}{U/B} — Instant
//! Face: Inundated Archive —  — Land
// IMPLEMENTED — the front face is a tutor to hand for an instant card or a
// card with flash (the reveal and the shuffle are derived from the search),
// and the back face is a land that enters tapped and taps for {U} or {B}.

use baylee_cards_dsl::prelude::*;

/// The back face's two colours, in the order the card prints them.
static BACK_COLORS: &[ManaColor] = &[ManaColor::Blue, ManaColor::Black];

/// `{T}: Add {U} or {B}.` — the back face's only ability.
static BACK_MANA: &[AbilityDef] = &[mana_ability!(&[Effect::mana_choice(BACK_COLORS)])];

card!(
    index = index::WATERLOGGED_TEACHINGS,
    oracle_id = "e6ad1be9-f13d-4590-b3db-e2d0fff46f03",
    scryfall_id = "060f9675-4921-4cbb-bae2-54c85c679fd4",
    color_identity = ColorSet::from_slice(&[Color::Black, Color::Blue]),
    faces = &[
        face!(
            name = "Waterlogged Teachings",
            mana_cost = mana!("{3}{U/B}"),
            types = TypeSet::INSTANT,
        ),
        face!(
            name = "Inundated Archive",
            types = TypeSet::LAND,
            enter_modifiers = &[EnterModifier::Tapped],
            abilities = BACK_MANA,
        ),
    ],
    coverage = Coverage::Implemented,
    abilities = &[spell!(&[Effect::SearchLibrary {
        filter: &Filter::Or(&[
            Filter::HasType(TypeSet::INSTANT),
            Filter::HasKeyword(KeywordSet::FLASH),
        ]),
        finds: &[Find::HAND],
        optional: false,
    }])],
);
