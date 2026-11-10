//! Arboria — {2}{G}{G} — World Enchantment
//! Oracle: Creatures can't attack a player unless that player cast a spell or put a nontoken permanent onto the battlefield during their last turn.
//! Set: DMR #149 — Dominaria Remastered | Scryfall ID: b18f4de2-adfe-4784-bd57-9e2ad9088825 | Oracle ID: acb3e93c-a1d3-458f-b8c3-c426cd359fa4
// IMPLEMENTED — creatures can't attack a player who neither cast a spell nor
// put a nontoken permanent onto the battlefield during their last turn.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::ARBORIA,
    oracle_id = "acb3e93c-a1d3-458f-b8c3-c426cd359fa4",
    scryfall_id = "b18f4de2-adfe-4784-bd57-9e2ad9088825",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Arboria",
        mana_cost = mana!("{2}{G}{G}"),
        types = TypeSet::ENCHANTMENT,
        supertypes = SupertypeSet::WORLD,
    ),],
    // "Creatures can't attack a player unless that player cast a spell or put
    // a nontoken permanent onto the battlefield during their last turn." A
    // planeswalker of that player can still be attacked (2022-12-08 ruling).
    abilities = &[static_ability!(
        Filter::CREATURE,
        Modifier::CantAttackPlayerIdleLastTurn
    )],
);
