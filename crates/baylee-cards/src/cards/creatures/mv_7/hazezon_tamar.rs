//! Hazezon Tamar — {4}{R}{G}{W} — Legendary Creature — Human Warrior
//! Oracle: When Hazezon enters, create X 1/1 Sand Warrior creature tokens that are red, green, and white at the beginning of your next upkeep, where X is the number of lands you control at that time.
//! Oracle: When Hazezon leaves the battlefield, exile all Sand Warriors.
//! Set: ME3 #151 — Masters Edition III | Scryfall ID: 4cd43773-2a6f-4f03-bcee-de32049561e5 | Oracle ID: d298df4e-7b60-4495-9773-cc81954b7dd9
// PARTIAL — the leaves-the-battlefield exile is on the card; the delayed
// token creation is not.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::HAZEZON_TAMAR,
    oracle_id = "d298df4e-7b60-4495-9773-cc81954b7dd9",
    scryfall_id = "4cd43773-2a6f-4f03-bcee-de32049561e5",
    color_identity = ColorSet::from_slice(&[Color::Green, Color::Red, Color::White]),
    commander = CommanderRule::Legendary,
    faces = &[face!(
        name = "Hazezon Tamar",
        mana_cost = mana!("{4}{R}{G}{W}"),
        types = TypeSet::CREATURE,
        supertypes = SupertypeSet::LEGENDARY,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::WARRIOR],
        power = Some(2),
        toughness = Some(4),
    ),],
    coverage = Coverage::Partial(
        "no Effect creates a delayed triggered ability at the beginning of \
         your next upkeep; DelayedWhen::NextUpkeep is reached only by \
         TransformSourceAtNextUpkeep and PayCostOrLoseLater, so the token \
         creation cannot be scheduled"
    ),
    // NOT SUPPORTED: "When Hazezon enters, create X 1/1 Sand Warrior
    // creature tokens that are red, green, and white at the beginning of
    // your next upkeep, where X is the number of lands you control at that
    // time." — no general `Effect` registers a delayed trigger for the next
    // upkeep window; only `TransformSourceAtNextUpkeep` and
    // `PayCostOrLoseLater` reach `DelayedWhen::NextUpkeep`.
    abilities = &[triggered!(
        Trigger::LeavesBattlefield(&Filter::This),
        &[Effect::ExileAll {
            filter: &Filter::And(&[
                Filter::HasSubtype(subtypes::creature::SAND),
                Filter::HasSubtype(subtypes::creature::WARRIOR)
            ])
        }]
    )],
);
