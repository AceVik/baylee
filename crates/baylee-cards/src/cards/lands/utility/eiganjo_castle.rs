//! Eiganjo Castle — (no cost) — Legendary Land
//! Oracle: {T}: Add {W}.
//! Oracle: {W}, {T}: Prevent the next 2 damage that would be dealt to target legendary creature this turn.
//! Set: CHK #275 — Champions of Kamigawa | Scryfall ID: 219c1d76-40cf-4edf-8145-e6cec8ca39ad | Oracle ID: 895a0e00-20a9-44f8-9215-66edcdf016b7
// IMPLEMENTED — {T}: Add {W}; the activated prevention ability is dropped (see NOT SUPPORTED).

use baylee_cards_dsl::prelude::*;

card!(
    index = index::EIGANJO_CASTLE,
    oracle_id = "895a0e00-20a9-44f8-9215-66edcdf016b7",
    scryfall_id = "219c1d76-40cf-4edf-8145-e6cec8ca39ad",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Eiganjo Castle",
        types = TypeSet::LAND,
        supertypes = SupertypeSet::LEGENDARY,
    ),],
    coverage = Coverage::Partial(
        "the {W}, {T} ability prevents a counted amount (the next 2) — \
         Modifier::PreventDamageToIt takes no amount and prevents all damage"
    ),
    abilities = &[mana_ability!(&[Effect::mana(ManaColor::White, 1)])],
);

// NOT SUPPORTED: "{W}, {T}: Prevent the next 2 damage that would be dealt to
// target legendary creature this turn." The only prevention the vocabulary has
// is Modifier::PreventDamageToIt (an *all*-damage shield, Maze of Ith's
// sentence) and Modifier::PreventDamageFromIt; neither carries an amount, and
// there is no Effect that prevents a counted amount. Written with the shield
// that exists, the card would prevent every point for the turn instead of the
// next two, so the ability comes off the card rather than being shipped wrong.
