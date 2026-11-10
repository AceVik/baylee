//! The Meathook Massacre — {X}{B}{B} — Legendary Enchantment
//! Oracle: When The Meathook Massacre enters, each creature gets -X/-X until end of turn.
//! Oracle: Whenever a creature you control dies, each opponent loses 1 life.
//! Oracle: Whenever a creature an opponent controls dies, you gain 1 life.
//! Set: INR #122 — Innistrad Remastered | Scryfall ID: 70d0540f-93c6-4af5-ab2d-65e6c03001c7 | Oracle ID: 127de52b-df75-4342-95a0-20d84c5bf916
// IMPLEMENTED — the enter trigger is a single -X/-X pump on every creature
// until end of turn, and the two dies-triggers drain each opponent and gain
// you life.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::THE_MEATHOOK_MASSACRE,
    oracle_id = "127de52b-df75-4342-95a0-20d84c5bf916",
    scryfall_id = "70d0540f-93c6-4af5-ab2d-65e6c03001c7",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "The Meathook Massacre",
        mana_cost = mana!("{X}{B}{B}"),
        types = TypeSet::ENCHANTMENT,
        supertypes = SupertypeSet::LEGENDARY,
    ),],
    coverage = Coverage::Implemented,
    abilities = &[
        triggered!(
            Trigger::ETB,
            &[Effect::PumpFilter {
                filter: &Filter::CREATURE,
                controlled_by: None,
                power: Amount::NegX,
                toughness: Amount::NegX,
                keywords: KeywordSet::EMPTY,
                duration: Duration::UntilEndOfTurn,
            }]
        ),
        triggered!(
            Trigger::Dies(&Filter::YOUR_CREATURE),
            &[Effect::LoseLife {
                amount: Amount::Fixed(1),
                target: PlayerRel::EachOpponent,
            }]
        ),
        triggered!(
            Trigger::Dies(&Filter::OPPONENT_CREATURE),
            &[Effect::gain_life(1)]
        ),
    ],
);
