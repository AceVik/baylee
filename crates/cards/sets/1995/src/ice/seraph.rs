//! Seraph — {6}{W} — Creature — Angel
//! Oracle: Flying
//! Oracle: Whenever a creature dealt damage by this creature this turn dies, put that card onto the battlefield under your control at the beginning of the next end step. Sacrifice the creature when you lose control of this creature.
//! Set: ME1 #26 — Masters Edition | Scryfall ID: b1c34789-3927-4547-b6fb-e121bacc7e78 | Oracle ID: a1c50bd1-120d-44ac-af20-13a90fc991e6
use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

// NOT SUPPORTED: "Whenever a creature dealt damage by this creature this turn dies, put that card onto the battlefield under your control at the beginning of the next end step. Sacrifice the creature when you lose control of this creature." Trigger::DiesAfterDamageByThis captures the pre-departure battlefield object version, and the engine has no delayed end-step graveyard return linked to a lose-control sacrifice trigger.
card!(
    index = index::SERAPH,
    oracle_id = "a1c50bd1-120d-44ac-af20-13a90fc991e6",
    scryfall_id = "b1c34789-3927-4547-b6fb-e121bacc7e78",
    color_identity = ColorSet::from_slice(&[Color::White]),
    keywords = KeywordSet::FLYING,
    coverage = Coverage::Partial(
        "has flying; the delayed end-step reanimation trigger when a creature dealt damage by Seraph this turn dies and the sacrifice-when-you-lose-control trigger are not supported",
    ),
    faces = &[face!(
        name = "Seraph",
        mana_cost = mana!("{6}{W}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::ANGEL],
        power = Some(4),
        toughness = Some(4),
    ),],
);
