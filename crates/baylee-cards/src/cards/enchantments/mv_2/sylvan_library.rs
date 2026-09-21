//! Sylvan Library — {1}{G} — Enchantment
//! Oracle: At the beginning of your draw step, you may draw two additional cards. If you do, choose two cards in your hand drawn this turn. For each of those cards, pay 4 life or put the card on top of your library.
//! Set: DMR #179 — Dominaria Remastered | Scryfall ID: 6ada256f-2e55-4c1f-b4d3-d7b10b498956 | Oracle ID: 92eed395-62ca-4293-882b-8565c40daab5
// PARTIAL — hand-written: the draw-step trigger and its "you may draw two additional cards" are built; the "If you do" sentence that pays for them is not expressible.

use baylee_cards_dsl::prelude::*;

// NOT SUPPORTED: "If you do, choose two cards in your hand drawn this turn. For
// each of those cards, pay 4 life or put the card on top of your library." — no
// `Filter` can name a card "drawn this turn" (the vocabulary has no such
// predicate and no per-turn drawn set), and nothing prices a choice in life:
// `Effect::PlayerMayPayOr` charges generic mana, and
// `Effect::PlayerMayPayCostOr` takes only the four parts a player answers by
// naming an object (Sacrifice, Discard, TapOther, ReturnToHand).

card!(
    index = index::SYLVAN_LIBRARY,
    oracle_id = "92eed395-62ca-4293-882b-8565c40daab5",
    scryfall_id = "6ada256f-2e55-4c1f-b4d3-d7b10b498956",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Sylvan Library",
        mana_cost = mana!("{1}{G}"),
        types = TypeSet::ENCHANTMENT,
    ),],
    coverage = Coverage::Partial(
        "no \"drawn this turn\" filter, and no effect offers 4 life as the price of a choice"
    ),
    abilities = &[triggered!(
        Trigger::StepBegin {
            step: StepKind::Draw,
            whose: PlayerRel::You,
        },
        &[Effect::MayDo {
            effects: &[Effect::draw(2)],
        }]
    )],
);
