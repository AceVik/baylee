//! Esper Sentinel — {W} — Artifact Creature — Human Soldier
//! Oracle: Whenever an opponent casts their first noncreature spell each turn, draw a card unless that player pays {X}, where X is this creature's power.
//! Set: MH2 #12 — Modern Horizons 2 | Scryfall ID: f3537373-ef54-4578-9d05-6216420ee349 | Oracle ID: 5def9f38-0a0b-4e8d-9f9d-29dcb46520b4
// IMPLEMENTED — first-noncreature-spell-per-turn tax (per-turn tracking),
// taxed at this creature's own power. It was written as a flat {1}, which is
// the right answer for a 1/1 with nothing on it and wrong the moment an
// anthem, a counter or an equipment touches it — the card prints {X} for
// exactly that reason.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes::creature;

static DRAW_ONE: Effect = Effect::DrawCards {
    amount: Amount::Fixed(1),
};

card!(
    index = 21468,
    oracle_id = "5def9f38-0a0b-4e8d-9f9d-29dcb46520b4",
    scryfall_id = "f3537373-ef54-4578-9d05-6216420ee349",
    faces = &[face!(
        name = "Esper Sentinel",
        mana_cost = mana!("{W}"),
        types = TypeSet::ARTIFACT.union(TypeSet::CREATURE),
        subtypes = &[creature::HUMAN, creature::SOLDIER],
        power = Some(1),
        toughness = Some(1),
    )],
    color_identity = ColorSet::from_slice(&[Color::White]),
    coverage = Coverage::Implemented,
    abilities = &[triggered!(
        Trigger::FirstNoncreatureSpellCast(PlayerRel::Opponent),
        &[Effect::PlayerMayPayOr {
            player: PlayerRel::Opponent,
            mana: Amount::SourcePower,
            effect: &DRAW_ONE,
        }]
    )],
);
