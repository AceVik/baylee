//! Riftstone Portal — (no cost) — Land
//! Oracle: {T}: Add {C}.
//! Oracle: As long as this card is in your graveyard, lands you control have "{T}: Add {G} or {W}."
//! Set: JUD #143 — Judgment | Scryfall ID: 92ece630-e484-4221-911f-e32048894f23 | Oracle ID: 8d7e05ba-5406-4d5e-bb8f-a4a6f3b0eaa7
use baylee_cards_dsl::prelude::*;
card!(
    index = index::RIFTSTONE_PORTAL,
    oracle_id = "8d7e05ba-5406-4d5e-bb8f-a4a6f3b0eaa7",
    scryfall_id = "92ece630-e484-4221-911f-e32048894f23",
    color_identity = ColorSet::from_slice(&[Color::Green, Color::White]),
    faces = &[face!(name = "Riftstone Portal", types = TypeSet::LAND)],
    coverage = Coverage::Implemented,
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        static_ability!(
            Filter::YOUR_LAND,
            Modifier::GrantActivated {
                cost: Cost::TAP,
                effects: &[Effect::mana_choice(&[ManaColor::Green, ManaColor::White])],
                mana_ability: true,
            },
            zone = ZoneRef::Graveyard
        ),
    ],
);
