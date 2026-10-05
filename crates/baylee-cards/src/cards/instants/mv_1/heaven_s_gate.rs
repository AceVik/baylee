//! Heaven's Gate — {W} — Instant
//! Oracle: One or more target creatures become white until end of turn.
//! Set: LEG #19 — Legends | Scryfall ID: 461d7c11-3a7d-42c2-bb6b-0a43779e6842 | Oracle ID: fafdba94-51cd-413c-82f3-eb294825c6ca
// PARTIAL — the whole spell is off the card.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::HEAVEN_S_GATE,
    oracle_id = "fafdba94-51cd-413c-82f3-eb294825c6ca",
    scryfall_id = "461d7c11-3a7d-42c2-bb6b-0a43779e6842",
    color_identity = ColorSet::from_slice(&[Color::White]),
    coverage = Coverage::Partial(
        "Effect::continuous with Filter::This registers its SetColor effect \
         against the resolution's first target only (subjects::this), and no \
         effect applies a modifier to every target of a spell"
    ),
    faces = &[face!(
        name = "Heaven's Gate",
        mana_cost = mana!("{W}"),
        types = TypeSet::INSTANT,
    ),],
    // NOT SUPPORTED: "One or more target creatures become white until end of
    // turn." — `Modifier::SetColor` is the layer-5 color change, but the only
    // door an effect uses for it is `Effect::continuous(&Filter::This, …)`,
    // and `Filter::This` in a resolution names the **first** target
    // (`subjects::this`), where `Effect::PumpTarget` is the effect that
    // reaches every target and carries P/T and keywords only. Coloring one
    // of several targets would be a different card.
    abilities = &[],
);
