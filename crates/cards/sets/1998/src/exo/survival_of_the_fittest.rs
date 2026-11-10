//! Survival of the Fittest — {1}{G} — Enchantment
//! Oracle: {G}, Discard a creature card: Search your library for a creature card, reveal that card, put it into your hand, then shuffle.
//! Set: TPR #199 — Tempest Remastered | Scryfall ID: 4ef0d7f9-ddb9-4e83-a9bf-09bec22fc80d | Oracle ID: 119d719d-e965-45b4-9bc9-ac03211b10c2
// IMPLEMENTED — a repeatable creature tutor into your hand, paid for by
// pitching a creature card. The reveal and the shuffle are the engine's own:
// a search narrower than "a card" that ends in a hidden zone shows what it
// found. The pool's one `CostPart::Discard`, so the hand-only menu
// `engine::cost_wizard` puts in front of the player is driven by this card
// alone — `Filter::CREATURE` says nothing about whose hand, and the rule
// (CR 701.9a) is what limits it to your own.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::SURVIVAL_OF_THE_FITTEST,
    oracle_id = "119d719d-e965-45b4-9bc9-ac03211b10c2",
    scryfall_id = "4ef0d7f9-ddb9-4e83-a9bf-09bec22fc80d",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Survival of the Fittest",
        mana_cost = mana!("{1}{G}"),
        types = TypeSet::ENCHANTMENT,
    ),],
    coverage = Coverage::Implemented,
    abilities = &[activated!(
        cost!("{G}", Discard(&Filter::CREATURE)),
        &[Effect::SearchLibrary {
            filter: &Filter::CREATURE,
            finds: &[Find::HAND],
            optional: false,
        }]
    )],
);
