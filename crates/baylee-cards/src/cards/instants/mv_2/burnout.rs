//! Burnout — {1}{R} — Instant
//! Oracle: Counter target instant spell if it's blue.
//! Oracle: Draw a card at the beginning of the next turn's upkeep.
//! Set: ME2 #121 — Masters Edition II | Scryfall ID: 8c8d78d3-6760-4a66-adbd-d8aa2544e057 | Oracle ID: 90865918-2f9a-4ab6-a881-e86f61a527cc

use baylee_cards_dsl::prelude::*;

static BLUE: Filter = Filter::HasColor(ColorSet::from_slice(&[Color::Blue]));

card!(
    index = index::BURNOUT,
    oracle_id = "90865918-2f9a-4ab6-a881-e86f61a527cc",
    scryfall_id = "8c8d78d3-6760-4a66-adbd-d8aa2544e057",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Burnout",
        mana_cost = mana!("{1}{R}"),
        types = TypeSet::INSTANT,
    ),],
    abilities = &[spell!(
        &[
            Effect::IfTargetMatches {
                filter: &BLUE,
                then: &[Effect::CounterTargetSpell],
            },
            Effect::AtNextTurnsUpkeep {
                effects: &[Effect::draw(1)],
            },
        ],
        targets = Some(TargetReq::one(TargetSpec::Spell(&Filter::HasType(
            TypeSet::INSTANT,
        )))),
    )],
);
