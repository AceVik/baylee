//! Suffocation — {1}{U} — Instant
//! Oracle: Cast this spell only if you were dealt damage this turn by a red instant or sorcery spell.
//! Oracle: Suffocation deals 4 damage to the controller of the last red instant or sorcery spell that dealt damage to you this turn.
//! Oracle: Draw a card at the beginning of the next turn's upkeep.
//! Set: ALL #38 — Alliances | Scryfall ID: d22104df-8147-45fd-897a-f99a815be062 | Oracle ID: 57aeb106-dc4f-4069-b596-f997263dc0cf

use baylee_cards_dsl::prelude::*;

card!(
    index = index::SUFFOCATION,
    oracle_id = "57aeb106-dc4f-4069-b596-f997263dc0cf",
    scryfall_id = "d22104df-8147-45fd-897a-f99a815be062",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Suffocation",
        mana_cost = mana!("{1}{U}"),
        types = TypeSet::INSTANT,
    ),],
    coverage = Coverage::Partial(
        "May be cast without having been dealt damage this turn by a red instant or sorcery spell, and deals 4 damage to each opponent (plus the next-turn's-upkeep card draw) rather than specifically to the controller of the last red instant or sorcery spell that dealt damage to you this turn."
    ),
    abilities = &[spell!(&[
        Effect::DealDamage {
            amount: Amount::Fixed(4),
            target: TargetSpec::Player(PlayerRel::EachOpponent),
        },
        Effect::AtNextTurnsUpkeep {
            effects: &[Effect::draw(1)],
        },
    ])],
);
