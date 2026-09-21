//! Garruk's Uprising — {2}{G} — Enchantment
//! Oracle: When this enchantment enters, if you control a creature with power 4 or greater, draw a card.
//! Oracle: Creatures you control have trample. (Each of those creatures can deal excess combat damage to the player or planeswalker it's attacking.)
//! Oracle: Whenever a creature you control with power 4 or greater enters, draw a card.
//! Set: ECC #109 — Lorwyn Eclipsed Commander | Scryfall ID: b58c4033-f764-42f6-966f-b7202a2babbf | Oracle ID: 3127ae9b-a7a7-43ec-89d7-688f8445b33d
// PARTIAL — the trample anthem, plus the enter trigger's draw read through
// Effect::IfEventPowerAtLeast (no filter compares power, so that check runs as
// the ability resolves). The enter-trigger's intervening if is dropped.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::GARRUK_S_UPRISING,
    oracle_id = "3127ae9b-a7a7-43ec-89d7-688f8445b33d",
    scryfall_id = "b58c4033-f764-42f6-966f-b7202a2babbf",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Partial(
        "no Filter compares power: the enter trigger's intervening if, \"if you control a creature with power 4 or greater\", has no Condition to be written with, and the second trigger's printed power restriction is read on resolution rather than on the trigger",
    ),
    faces = &[face!(
        name = "Garruk's Uprising",
        mana_cost = mana!("{2}{G}"),
        types = TypeSet::ENCHANTMENT,
    ),],
    abilities = &[
        // NOT SUPPORTED: When this enchantment enters, if you control a
        // creature with power 4 or greater, draw a card. — the intervening if
        // (CR 603.4) needs a Condition, and Condition::ControlCount would need
        // a Filter that compares power, which the DSL does not have.
        static_ability!(
            Filter::YOUR_CREATURE,
            Modifier::AddKeyword(KeywordSet::TRAMPLE)
        ),
        triggered!(
            Trigger::EntersBattlefield(&Filter::YOUR_CREATURE),
            &[Effect::IfEventPowerAtLeast {
                n: 4,
                then: &[Effect::draw(1)],
                otherwise: &[],
            }]
        ),
    ],
);
