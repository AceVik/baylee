//! Gaea's Cradle — (no cost) — Legendary Land
//! Oracle: {T}: Add {G} for each creature you control.
//! Set: USG #321 — Urza's Saga | Scryfall ID: 25b0b816-0583-44aa-9dc5-f3ff48993a51 | Oracle ID: 7c427c3d-ecd8-45ef-bebd-8f10f4a311db
// IMPLEMENTED — dynamic green mana ({G} for each creature you control).

use baylee_cards_dsl::prelude::*;

card!(
    index = index::GAEA_S_CRADLE,
    oracle_id = "7c427c3d-ecd8-45ef-bebd-8f10f4a311db",
    scryfall_id = "25b0b816-0583-44aa-9dc5-f3ff48993a51",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Gaea's Cradle",
        types = TypeSet::LAND,
        supertypes = SupertypeSet::LEGENDARY,
    ),],
    coverage = Coverage::Implemented,
    abilities = &[mana_ability!(&[Effect::mana_dynamic(
        ManaColor::Green,
        Amount::CountOf {
            filter: &Filter::YOUR_CREATURE,
            zone: ZoneSel::Battlefield,
        },
    )])],
);
