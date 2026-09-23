//! Garruk's Uprising — {2}{G} — Enchantment
//! Oracle: When this enchantment enters, if you control a creature with power 4 or greater, draw a card.
//! Oracle: Creatures you control have trample. (Each of those creatures can deal excess combat damage to the player or planeswalker it's attacking.)
//! Oracle: Whenever a creature you control with power 4 or greater enters, draw a card.
//! Set: ECC #109 — Lorwyn Eclipsed Commander | Scryfall ID: b58c4033-f764-42f6-966f-b7202a2babbf | Oracle ID: 3127ae9b-a7a7-43ec-89d7-688f8445b33d
// IMPLEMENTED — all three lines. The same sentence appears twice on this
// card in two different rules positions, which is why it is one filter used
// twice rather than two spellings.

use baylee_cards_dsl::prelude::*;

/// Read twice by this card, and that is the point of naming it: the enter
/// trigger asks it as an intervening `if` (CR 603.4 — checked when the
/// ability would trigger *and* again on resolution), and the second trigger
/// asks it of the creature that entered (CR 603.2 — an event that does not
/// match causes no trigger at all). Two rules, one sentence; a card that
/// spelled them differently would be a card whose two halves disagree about
/// what "power 4 or greater" means.
static BIG_CREATURE_YOU_CONTROL: Filter =
    Filter::And(&[Filter::YOUR_CREATURE, Filter::PowerAtLeast(4)]);

card!(
    index = index::GARRUK_S_UPRISING,
    oracle_id = "3127ae9b-a7a7-43ec-89d7-688f8445b33d",
    scryfall_id = "b58c4033-f764-42f6-966f-b7202a2babbf",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Garruk's Uprising",
        mana_cost = mana!("{2}{G}"),
        types = TypeSet::ENCHANTMENT,
    ),],
    abilities = &[
        triggered!(
            Trigger::ETB,
            &[Effect::draw(1)],
            condition = Some(Condition::ControlCount(&BIG_CREATURE_YOU_CONTROL, 1)),
        ),
        static_ability!(
            Filter::YOUR_CREATURE,
            Modifier::AddKeyword(KeywordSet::TRAMPLE)
        ),
        triggered!(
            Trigger::EntersBattlefield(&BIG_CREATURE_YOU_CONTROL),
            &[Effect::draw(1)]
        ),
    ],
);
