//! Telim'Tor's Edict — {R} — Instant
//! Oracle: Exile target permanent you own or control.
//! Oracle: Draw a card at the beginning of the next turn's upkeep.
//! Set: MIR #198 — Mirage | Scryfall ID: 088a60f0-2224-4e00-a06a-1d376c8d82a4 | Oracle ID: 0e768d03-b2a3-4783-9cf5-7470f0203349
// IMPLEMENTED — exiles target permanent you own or control and draws a card at the next turn's upkeep.

use baylee_cards_dsl::prelude::*;

static TARGET1: Filter = Filter::Or(&[Filter::ControlledByYou, Filter::OwnedByYou]);

card!(
    index = index::TELIM_TOR_S_EDICT,
    oracle_id = "0e768d03-b2a3-4783-9cf5-7470f0203349",
    scryfall_id = "088a60f0-2224-4e00-a06a-1d376c8d82a4",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Telim'Tor's Edict",
        mana_cost = mana!("{R}"),
        types = TypeSet::INSTANT,
    ),],
    abilities = &[spell!(
        &[
            Effect::exile(TargetSpec::Object(&TARGET1)),
            Effect::AtNextTurnsUpkeep {
                effects: &[Effect::draw(1)]
            }
        ],
        targets = Some(TargetReq::one(TargetSpec::Object(&TARGET1)))
    ),],
);
