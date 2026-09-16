//! Survival of the Fittest — {1}{G} — Enchantment
//! Oracle: {G}, Discard a creature card: Search your library for a creature card, reveal that card, put it into your hand, then shuffle.
//! Set: TPR #199 — Tempest Remastered | Scryfall ID: 4ef0d7f9-ddb9-4e83-a9bf-09bec22fc80d | Oracle ID: 119d719d-e965-45b4-9bc9-ac03211b10c2
// PARTIAL — a repeatable creature tutor into your hand, paid for by pitching
// a creature card. The reveal and the shuffle are the engine's own: a search
// narrower than "a card" that ends in a hidden zone shows what it found.
// NOT SUPPORTED: `Discard a creature card` as part of the cost. A cost that
// names something to *choose* has nothing to ask the question with —
// `pay_cost` refuses `CostPart::Discard` outright — so the ability is
// unpayable and `can_afford` therefore declines to offer it at all. The card
// plays exactly as though the line were not printed, which is what `Partial`
// promises about it; it goes to `Implemented` the day an activation can
// suspend on a choice during cost payment, the way the cast wizard already
// does for `ExileFromHand`.

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
    coverage = Coverage::Partial("a discard cost cannot be chosen during an activation"),
    abilities = &[activated!(
        cost!("{G}", Discard(&Filter::CREATURE)),
        &[Effect::SearchLibrary {
            filter: &Filter::CREATURE,
            finds: &[Find::HAND],
            optional: false,
        }]
    )],
);
