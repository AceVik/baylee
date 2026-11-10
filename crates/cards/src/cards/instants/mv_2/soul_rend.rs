//! Soul Rend — {1}{B} — Instant
//! Oracle: Destroy target creature if it's white. A creature destroyed this way can't be regenerated.
//! Oracle: Draw a card at the beginning of the next turn's upkeep.
//! Set: MIR #144 — Mirage | Scryfall ID: 7fa084e1-05c2-4691-b9fe-3e3c717e5c9d | Oracle ID: 80d233cc-7748-41a2-bfd6-2189d0136900
// IMPLEMENTED — destroys target creature if it's white without regeneration and draws a card at the next turn's upkeep.

use baylee_cards_dsl::prelude::*;

static WHITE: Filter = Filter::HasColor(ColorSet::from_slice(&[Color::White]));

card!(
    index = index::SOUL_REND,
    oracle_id = "80d233cc-7748-41a2-bfd6-2189d0136900",
    scryfall_id = "7fa084e1-05c2-4691-b9fe-3e3c717e5c9d",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Soul Rend",
        mana_cost = mana!("{1}{B}"),
        types = TypeSet::INSTANT,
    ),],
    abilities = &[spell!(
        &[
            Effect::IfTargetMatches {
                filter: &WHITE,
                then: &[Effect::destroy_no_regen(TargetSpec::Object(
                    &Filter::CREATURE
                ))],
            },
            Effect::AtNextTurnsUpkeep {
                effects: &[Effect::draw(1)],
            },
        ],
        targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
    )],
);
