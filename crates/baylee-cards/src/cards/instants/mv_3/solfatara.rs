//! Solfatara — {2}{R} — Instant
//! Oracle: Target player can't play lands this turn.
//! Oracle: Draw a card at the beginning of the next turn's upkeep.
//! Set: VIS #93 — Visions | Scryfall ID: c5d4bd6f-b019-4594-aa41-138fa58ba529 | Oracle ID: 6053a192-fcf9-4b06-9f46-e84eaadaa882
// PARTIAL: targets a player, prevents land plays this turn, and draws a card at the beginning of the next turn's upkeep; scoping Modifier::PlayersCantPlay solely to the targeted player rather than all players is not yet modeled.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::SOLFATARA,
    oracle_id = "6053a192-fcf9-4b06-9f46-e84eaadaa882",
    scryfall_id = "c5d4bd6f-b019-4594-aa41-138fa58ba529",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Partial(
        "targets a player, prevents land plays this turn, and draws a card at the beginning of the next turn's upkeep; scoping Modifier::PlayersCantPlay solely to the targeted player rather than all players is not yet modeled",
    ),
    faces = &[face!(
        name = "Solfatara",
        mana_cost = mana!("{2}{R}"),
        types = TypeSet::INSTANT,
    ),],
    abilities = &[spell!(
        &[
            Effect::continuous(
                &Filter::Any,
                Modifier::PlayersCantPlay(&Filter::LAND),
                Duration::UntilEndOfTurn,
            ),
            Effect::AtNextTurnsUpkeep {
                effects: &[Effect::draw(1)],
            },
        ],
        targets = Some(TargetReq::one(TargetSpec::AnyPlayer)),
    )],
);
