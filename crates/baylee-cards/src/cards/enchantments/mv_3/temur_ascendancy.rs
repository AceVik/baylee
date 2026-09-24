//! Temur Ascendancy — {G}{U}{R} — Enchantment
//! Oracle: Creatures you control have haste.
//! Oracle: Whenever a creature you control with power 4 or greater enters, you may draw a card.
//! Set: TDC #305 — Tarkir: Dragonstorm Commander | Scryfall ID: 5cedb54a-a6f6-48aa-acdf-01c988c1c37d | Oracle ID: e68dc47c-692f-4420-9799-eee104017273
// IMPLEMENTED — the haste anthem, and the enter trigger whose subject is now
// sayable: the restriction sits on the trigger, so a 2/2 puts nothing on the
// stack at all.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::TEMUR_ASCENDANCY,
    oracle_id = "e68dc47c-692f-4420-9799-eee104017273",
    scryfall_id = "5cedb54a-a6f6-48aa-acdf-01c988c1c37d",
    color_identity = ColorSet::from_slice(&[Color::Green, Color::Red, Color::Blue]),
    faces = &[face!(
        name = "Temur Ascendancy",
        mana_cost = mana!("{G}{U}{R}"),
        types = TypeSet::ENCHANTMENT,
    ),],
    coverage = Coverage::Implemented,
    abilities = &[
        static_ability!(
            Filter::YOUR_CREATURE,
            Modifier::AddKeyword(KeywordSet::HASTE)
        ),
        // The restriction belongs to the **trigger's subject**, not to its
        // effect: written as "any creature you control enters, then check
        // the power", the ability would go on the stack for every creature
        // and be visible to every opponent as a thing that happened. CR 603.2
        // is what makes the difference — a trigger whose event does not match
        // simply does not trigger.
        triggered!(
            Trigger::EntersBattlefield(&Filter::YOUR_CREATURE_WITH_POWER_4_OR_GREATER),
            &[Effect::MayDo {
                effects: &[Effect::draw(1)],
            }]
        ),
    ],
);
