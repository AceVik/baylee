//! Trenzalore Clocktower — (no cost) — Legendary Land
//! Oracle: {T}: Add {U}. Put a time counter on Trenzalore Clocktower.
//! Oracle: {1}{U}, {T}, Remove twelve time counters from Trenzalore Clocktower and exile it: Shuffle your graveyard and hand into your library, then draw seven cards. Activate only if you control a Time Lord.
//! Set: WHO #190 — Doctor Who | Scryfall ID: 64444549-1198-4465-91d5-02a7903691dc | Oracle ID: 69143645-97b6-4c7c-9fa2-844fb3b99822
// PARTIAL — {T} adds {U} and a time counter; the second ability's cost
// ({1}{U}, {T}, twelve time counters, exile itself), its Time Lord
// condition, the graveyard shuffle and the draw seven are all written. The
// printed "and hand" is not: nothing in the DSL moves a hand into a library.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes::creature;

static TIME_LORD: Filter = Filter::HasSubtype(creature::TIME_LORD);

card!(
    index = index::TRENZALORE_CLOCKTOWER,
    oracle_id = "69143645-97b6-4c7c-9fa2-844fb3b99822",
    scryfall_id = "64444549-1198-4465-91d5-02a7903691dc",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Partial(
        "the second ability shuffles the hand into the library as well as the graveyard, and no effect moves a hand into a library"
    ),
    faces = &[face!(
        name = "Trenzalore Clocktower",
        types = TypeSet::LAND,
        supertypes = SupertypeSet::LEGENDARY,
    ),],
    abilities = &[
        mana_ability!(&[
            Effect::mana(ManaColor::Blue, 1),
            Effect::AddCounter {
                kind: CounterKind::Time,
                amount: Amount::Fixed(1),
            },
        ]),
        // NOT SUPPORTED: "Shuffle your graveyard and hand into your library" —
        // Effect::ShuffleGraveyardIntoLibrary takes the graveyard alone; no
        // variant moves a hand into a library.
        activated!(
            cost!(
                "{1}{U}",
                TapSelf,
                RemoveCounterSelf {
                    kind: CounterKind::Time,
                    n: 12
                },
                ExileSelf,
            ),
            &[Effect::ShuffleGraveyardIntoLibrary, Effect::draw(7)],
            condition = Some(Condition::ControlCount(&TIME_LORD, 1)),
        ),
    ],
);
