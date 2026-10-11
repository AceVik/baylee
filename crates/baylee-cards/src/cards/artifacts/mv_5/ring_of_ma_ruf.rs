//! Ring of Ma'rûf — {5} — Artifact
//! Oracle: {5}, {T}, Exile this artifact: The next time you would draw a card this turn, instead put a card you own from outside the game into your hand.
//! Set: ME1 #163 — Masters Edition | Scryfall ID: fa740755-244f-4658-a9e2-aa4cf6742808 | Oracle ID: c79b9187-cbfe-43a0-bdc8-4f7e0d215607

use baylee_cards_dsl::prelude::*;

card!(
    index = index::RING_OF_MA_RUF,
    oracle_id = "c79b9187-cbfe-43a0-bdc8-4f7e0d215607",
    scryfall_id = "fa740755-244f-4658-a9e2-aa4cf6742808",
    faces = &[face!(
        name = "Ring of Ma'rûf",
        mana_cost = mana!("{5}"),
        types = TypeSet::ARTIFACT,
        abilities = &[activated!(
            cost!("{5}", TapSelf, ExileSelf),
            &[Effect::continuous(
                &Filter::Any,
                Modifier::NextDrawWish,
                Duration::UntilEndOfTurn
            )]
        )],
    ),],
    coverage = Coverage::Implemented,
);
