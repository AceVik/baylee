//! Elvish Spirit Guide — {2}{G} — Creature — Elf Spirit
//! Oracle: Exile this card from your hand: Add {G}.
//! Set: DMR #157 — Dominaria Remastered | Scryfall ID: 50aded92-f934-441f-8e48-d68d471e70d8 | Oracle ID: 6b0e23cf-7d68-4329-86db-7adc26abd86b

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::ELVISH_SPIRIT_GUIDE,
    oracle_id = "6b0e23cf-7d68-4329-86db-7adc26abd86b",
    scryfall_id = "50aded92-f934-441f-8e48-d68d471e70d8",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Elvish Spirit Guide",
        mana_cost = mana!("{2}{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::ELF, subtypes::creature::SPIRIT],
        power = Some(2),
        toughness = Some(2),
    ),],
    abilities = &[mana_ability!(
        cost!(ExileSelf),
        &[Effect::mana(ManaColor::Green, 1)],
        zone = ActivationZone::Hand,
    )],
);
