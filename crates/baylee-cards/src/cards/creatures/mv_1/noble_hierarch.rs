//! Noble Hierarch — {G} — Creature — Human Druid
//! Oracle: Exalted (Whenever a creature you control attacks alone, that creature gets +1/+1 until end of turn.)
//! Oracle: {T}: Add {G}, {W}, or {U}.
//! Set: 2XM #177 — Double Masters | Scryfall ID: 400382a4-aea2-4827-b06a-1b0b3745908b | Oracle ID: 98aa9424-5912-4bd6-9300-b3972a31d8af
// PARTIAL — the three-colour mana ability is built; Exalted is not
// expressible (see the NOT SUPPORTED note in `abilities`).

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::NOBLE_HIERARCH,
    oracle_id = "98aa9424-5912-4bd6-9300-b3972a31d8af",
    scryfall_id = "400382a4-aea2-4827-b06a-1b0b3745908b",
    color_identity = ColorSet::from_slice(&[Color::Green, Color::Blue, Color::White]),
    faces = &[face!(
        name = "Noble Hierarch",
        mana_cost = mana!("{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::DRUID],
        power = Some(0),
        toughness = Some(1),
    ),],
    coverage = Coverage::Partial(
        "Exalted: no trigger can say \"attacks alone\" and exalted is not a keyword bit the engine reads"
    ),
    abilities = &[
        // NOT SUPPORTED: Exalted — "Whenever a creature you control attacks
        // alone, that creature gets +1/+1 until end of turn."
        // Trigger::Attacks(&Filter::YOUR_CREATURE) fires for every attacker
        // and no Condition and no Filter can ask whether it attacked *alone*
        // (nothing counts attackers, and there is no "the lone attacker"
        // filter), so a pump here would be an anthem for the whole team
        // rather than one +1/+1. Exalted is also not one of the keyword bits
        // the engine reads, so it cannot be carried as a keyword either.
        mana_ability!(&[Effect::mana_choice(&[
            ManaColor::Green,
            ManaColor::White,
            ManaColor::Blue,
        ])]),
    ],
);
