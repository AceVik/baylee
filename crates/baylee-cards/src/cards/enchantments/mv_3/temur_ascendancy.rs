//! Temur Ascendancy — {G}{U}{R} — Enchantment
//! Oracle: Creatures you control have haste.
//! Oracle: Whenever a creature you control with power 4 or greater enters, you may draw a card.
//! Set: TDC #305 — Tarkir: Dragonstorm Commander | Scryfall ID: 5cedb54a-a6f6-48aa-acdf-01c988c1c37d | Oracle ID: e68dc47c-692f-4420-9799-eee104017273
// PARTIAL — creatures you control have haste; the enter trigger is left off,
// because the DSL has no filter that compares power (see the NOT SUPPORTED line).

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
    coverage = Coverage::Partial(
        "the trigger's subject — \"a creature you control with power 4 or greater\" — has no Filter that compares power",
    ),
    abilities = &[
        static_ability!(
            Filter::YOUR_CREATURE,
            Modifier::AddKeyword(KeywordSet::HASTE)
        ),
        // NOT SUPPORTED: "Whenever a creature you control with power 4 or
        // greater enters, you may draw a card." — the restriction sits on the
        // trigger's subject, and Filter has CmcAtMost / CmcAtLeast /
        // ToughnessAtMost and no power comparison. The nearest spelling,
        // Trigger::EntersBattlefield(&Filter::YOUR_CREATURE) with
        // Effect::IfEventPowerAtLeast inside, would put the ability on the
        // stack for every creature you control, which the card does not do.
    ],
);
