//! Riftstone Portal — (no cost) — Land
//! Oracle: {T}: Add {C}.
//! Oracle: As long as this card is in your graveyard, lands you control have "{T}: Add {G} or {W}."
//! Set: JUD #143 — Judgment | Scryfall ID: 92ece630-e484-4221-911f-e32048894f23 | Oracle ID: 8d7e05ba-5406-4d5e-bb8f-a4a6f3b0eaa7
// PARTIAL — the printed {T}: Add {C} is implemented; the graveyard clause
// cannot be said at all (see below), so the land taps for colorless mana and
// nothing else.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::RIFTSTONE_PORTAL,
    oracle_id = "8d7e05ba-5406-4d5e-bb8f-a4a6f3b0eaa7",
    scryfall_id = "92ece630-e484-4221-911f-e32048894f23",
    color_identity = ColorSet::from_slice(&[Color::Green, Color::White]),
    faces = &[face!(name = "Riftstone Portal", types = TypeSet::LAND,),],
    coverage = Coverage::Partial(
        "the graveyard clause is a static that grants lands you control an \
         activated ability: StaticAbility has no zone to function from (its \
         source is only read while it is a permanent on the battlefield) and \
         ability-granting statics are not supported yet"
    ),
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        // NOT SUPPORTED: As long as this card is in your graveyard, lands you
        // control have "{T}: Add {G} or {W}." — an ability-granting static
        // functioning from the graveyard; `Modifier::GrantActivated` is not
        // enforced (see "Ability-granting statics" in docs/card-dsl.md, M3+)
        // and no `StaticAbility` says which zone its source functions from.
    ],
);
