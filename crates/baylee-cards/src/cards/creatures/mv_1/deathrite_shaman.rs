//! Deathrite Shaman — {B/G} — Creature — Elf Shaman
//! Oracle: {T}: Exile target land card from a graveyard. Add one mana of any color. (Activate only as an instant.)
//! Oracle: {B}, {T}: Exile target instant or sorcery card from a graveyard. Each opponent loses 2 life.
//! Oracle: {G}, {T}: Exile target creature card from a graveyard. You gain 2 life.
//! Set: RVR #175 — Ravnica Remastered | Scryfall ID: cfdb1c47-14be-491f-88b3-bed03489dbc5 | Oracle ID: 22f1a4a4-c423-4d1c-8775-0ed604a9fa51
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::DEATHRITE_SHAMAN,
    oracle_id = "22f1a4a4-c423-4d1c-8775-0ed604a9fa51",
    scryfall_id = "cfdb1c47-14be-491f-88b3-bed03489dbc5",
    color_identity = ColorSet::from_slice(&[Color::Black, Color::Green]),
    faces = &[face!(
        name = "Deathrite Shaman",
        mana_cost = mana!("{B/G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::ELF, subtypes::creature::SHAMAN],
        power = Some(1),
        toughness = Some(2),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
