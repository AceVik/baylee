//! Urza's Fun House — (no cost) — Land — Urza's
//! Oracle: {T}: Add {C}.
//! Oracle: {T}: Add {∞}. Activate only once and only if you control an Urza's Mine, an Urza's Power-Plant, and an Urza's Tower.
//! Oracle: {7}, {T}: Head to AskUrza.com and click Urza's Fun House.
//! Set: UNF #199 — Unfinity | Scryfall ID: 42c47a99-965c-4bde-935a-10f28cf5ccba | Oracle ID: 6538fb05-7cc6-4bb9-9b57-11f4f07b5e59
// PARTIAL — {T}: Add {C} is built; the Tron mana ability and the AskUrza
// ability are not expressible and are left off, see the NOT SUPPORTED lines.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::URZA_S_FUN_HOUSE,
    oracle_id = "6538fb05-7cc6-4bb9-9b57-11f4f07b5e59",
    scryfall_id = "42c47a99-965c-4bde-935a-10f28cf5ccba",
    faces = &[face!(
        name = "Urza's Fun House",
        types = TypeSet::LAND,
        subtypes = &[subtypes::land::URZA_S],
    ),],
    coverage = Coverage::Partial(
        "the Tron mana ability — no Amount is infinite, ActivationLimit has no \
         per-game variant, and Condition::ControlCount counts one filter rather \
         than three differently-named lands — and the AskUrza.com ability, an \
         action outside the game"
    ),
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        // NOT SUPPORTED: "{T}: Add {∞}. Activate only once and only if you
        // control an Urza's Mine, an Urza's Power-Plant, and an Urza's Tower."
        // Every Amount is a magnitude the engine can evaluate and none of them
        // is endless; "only once" is a limit per *game* where ActivationLimit
        // offers only Unlimited and PerTurn(u8); and the requirement names
        // three *different* lands, which a Condition::ControlCount over one
        // filter cannot express (three Urza's Mines would satisfy it).
        // NOT SUPPORTED: "{7}, {T}: Head to AskUrza.com and click Urza's Fun
        // House." An action taken outside the game, with no DSL variant at all.
    ],
);
