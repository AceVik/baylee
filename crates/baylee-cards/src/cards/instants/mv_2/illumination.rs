//! Illumination — {W}{W} — Instant
//! Oracle: Counter target artifact or enchantment spell. Its controller gains life equal to its mana value.
//! Set: MIR #21 — Mirage | Scryfall ID: eb28f6e5-c9ef-416e-b315-967d857e7600 | Oracle ID: cd1a6508-aaba-4515-9496-057915d02040
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;

static ARTIFACT_OR_ENCHANTMENT: Filter = Filter::Or(&[Filter::ARTIFACT, Filter::ENCHANTMENT]);

card!(
    index = index::ILLUMINATION,
    oracle_id = "cd1a6508-aaba-4515-9496-057915d02040",
    scryfall_id = "eb28f6e5-c9ef-416e-b315-967d857e7600",
    color_identity = ColorSet::from_slice(&[Color::White]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Illumination",
        mana_cost = mana!("{W}{W}"),
        types = TypeSet::INSTANT,
    ),],
    abilities = &[spell!(
        &[
            Effect::CounterTargetSpell,
            Effect::GainLifeFor {
                amount: Amount::TargetCmc,
                who: PlayerRel::ControllerOfTarget,
            },
        ],
        targets = Some(TargetReq::one(TargetSpec::Spell(&ARTIFACT_OR_ENCHANTMENT))),
    )],
);
