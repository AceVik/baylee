//! Noble Hierarch — {G} — Creature — Human Druid
//! Oracle: Exalted (Whenever a creature you control attacks alone, that creature gets +1/+1 until end of turn.)
//! Oracle: {T}: Add {G}, {W}, or {U}.
//! Set: 2XM #177 — Double Masters | Scryfall ID: 400382a4-aea2-4827-b06a-1b0b3745908b | Oracle ID: 98aa9424-5912-4bd6-9300-b3972a31d8af
// IMPLEMENTED — mana choice and exalted.

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
    coverage = Coverage::Implemented,
    abilities = &[
        mana_ability!(&[Effect::mana_choice(&[
            ManaColor::Green,
            ManaColor::White,
            ManaColor::Blue,
        ])]),
        triggered!(
            Trigger::AttacksAlone(&Filter::YOUR_CREATURE),
            &[Effect::continuous(
                &Filter::This,
                Modifier::ModifyPT(1, 1),
                Duration::UntilEndOfTurn
            )],
            targets = Some(TargetReq::one(TargetSpec::EventObject)),
        ),
    ],
);
