//! Solfatara — {2}{R} — Instant
//! Oracle: Target player can't play lands this turn.
//! Oracle: Draw a card at the beginning of the next turn's upkeep.
//! Set: VIS #93 — Visions | Scryfall ID: c5d4bd6f-b019-4594-aa41-138fa58ba529 | Oracle ID: 6053a192-fcf9-4b06-9f46-e84eaadaa882
// PARTIAL: draws a card at the beginning of the next turn's upkeep; the first sentence (target player can't play lands this turn) is left out, because a land-play lock cannot be scoped to one player and would stop every player from playing lands, including the caster.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::SOLFATARA,
    oracle_id = "6053a192-fcf9-4b06-9f46-e84eaadaa882",
    scryfall_id = "c5d4bd6f-b019-4594-aa41-138fa58ba529",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Partial(
        "draws a card at the beginning of the next turn's upkeep; the first sentence (target player can't play lands this turn) is left out, because a land-play lock cannot be scoped to one player and would stop every player from playing lands, including the caster",
    ),
    faces = &[face!(
        name = "Solfatara",
        mana_cost = mana!("{2}{R}"),
        types = TypeSet::INSTANT,
    ),],
    abilities = &[spell!(&[
        // NOT SUPPORTED: "Target player can't play lands this turn." The only land-play lock,
        // `Modifier::PlayersCantPlay`, binds all players; aimed at oneself on an opponent's turn
        // it would lock that opponent instead, which is not what the target decides.
        Effect::AtNextTurnsUpkeep {
            effects: &[Effect::draw(1)],
        },
    ])],
);
