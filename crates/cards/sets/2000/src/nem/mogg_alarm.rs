//! Mogg Alarm — {1}{R}{R} — Sorcery
//! Oracle: You may sacrifice two Mountains rather than pay this spell's mana cost.
//! Oracle: Create two 1/1 red Goblin creature tokens.
//! Set: NEM #93 — Nemesis | Scryfall ID: f246e128-0a43-478a-a232-51020fab76d5 | Oracle ID: 1c2117d5-4600-489c-be5b-dc0ed69b30ca
// PARTIAL — create two 1/1 red Goblin creature tokens.
// NOT SUPPORTED: alternative cost of sacrificing two Mountains (`cast_wizard::paid_as_an_alternative_cost` only supports `PayLife` and `ExileFromHand`).

use baylee_cards_dsl::prelude::*;

use crate::generated_tokens;

card!(
    index = index::MOGG_ALARM,
    oracle_id = "1c2117d5-4600-489c-be5b-dc0ed69b30ca",
    scryfall_id = "f246e128-0a43-478a-a232-51020fab76d5",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Partial(
        "cast_wizard::paid_as_an_alternative_cost only supports PayLife and ExileFromHand, not sacrificing two Mountains",
    ),
    faces = &[face!(
        name = "Mogg Alarm",
        mana_cost = mana!("{1}{R}{R}"),
        types = TypeSet::SORCERY,
    ),],
    abilities = &[spell!(&[Effect::CreateTokenN {
        token: &generated_tokens::GOBLIN_1_1_RED,
        amount: Amount::Fixed(2),
    }]),],
);
