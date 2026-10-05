//! Sea Kings' Blessing — {U} — Instant
//! Oracle: One or more target creatures become blue until end of turn.
//! Set: LEG #75 — Legends | Scryfall ID: 11d1f02d-533e-4b77-a72a-ff5f91ae0626 | Oracle ID: e6ea475e-61ba-487b-b7e9-683845a90b73
// PARTIAL — the whole spell is off the card.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::SEA_KINGS_BLESSING,
    oracle_id = "e6ea475e-61ba-487b-b7e9-683845a90b73",
    scryfall_id = "11d1f02d-533e-4b77-a72a-ff5f91ae0626",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Partial(
        "Effect::continuous with Filter::This registers its SetColor effect \
         against the resolution's first target only (subjects::this), and no \
         effect applies a modifier to every target of a spell"
    ),
    faces = &[face!(
        name = "Sea Kings' Blessing",
        mana_cost = mana!("{U}"),
        types = TypeSet::INSTANT,
    ),],
    // NOT SUPPORTED: "One or more target creatures become blue until end of
    // turn." — `Modifier::SetColor` is the layer-5 color change, but the only
    // door an effect uses for it is `Effect::continuous(&Filter::This, …)`,
    // and `Filter::This` in a resolution names the **first** target
    // (`subjects::this`), where `Effect::PumpTarget` is the effect that
    // reaches every target and carries P/T and keywords only. Coloring one
    // of several targets would be a different card.
    abilities = &[],
);
