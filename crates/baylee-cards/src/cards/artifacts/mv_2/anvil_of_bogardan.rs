//! Anvil of Bogardan — {2} — Artifact
//! Oracle: Players have no maximum hand size.
//! Oracle: At the beginning of each player's draw step, that player draws an additional card, then discards a card.
//! Set: VIS #141 — Visions | Scryfall ID: 7ff965dd-54b4-4f21-a52f-81c0dd1e691e | Oracle ID: 9deb2a0b-f40c-4d13-8321-41dc41448d24
// PARTIAL — grants no maximum hand size to its controller and makes the active player draw an additional card then discard a card at the beginning of each player's draw step; no all-players NoMaxHandSize modifier ('Players have no maximum hand size') exists yet.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::ANVIL_OF_BOGARDAN,
    oracle_id = "9deb2a0b-f40c-4d13-8321-41dc41448d24",
    scryfall_id = "7ff965dd-54b4-4f21-a52f-81c0dd1e691e",
    coverage = Coverage::Partial(
        "grants no maximum hand size to its controller and makes the active player draw an additional card then discard a card at the beginning of each player's draw step; no all-players NoMaxHandSize modifier ('Players have no maximum hand size') exists yet",
    ),
    faces = &[face!(
        name = "Anvil of Bogardan",
        mana_cost = mana!("{2}"),
        types = TypeSet::ARTIFACT,
    ),],
    abilities = &[
        static_ability!(Filter::Any, Modifier::NoMaxHandSize),
        triggered!(
            Trigger::StepBegin {
                step: StepKind::Draw,
                whose: PlayerRel::EachPlayer,
            },
            &[
                Effect::DrawCardsFor {
                    amount: Amount::Fixed(1),
                    who: PlayerRel::ActivePlayer,
                },
                Effect::DiscardForPlayers {
                    who: PlayerRel::ActivePlayer,
                    count: 1,
                },
            ],
        ),
    ],
);
