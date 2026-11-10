//! Crumble — {G} — Instant
//! Oracle: Destroy target artifact. It can't be regenerated. That artifact's controller gains life equal to its mana value.
//! Set: ME4 #147 — Masters Edition IV | Scryfall ID: 4283c407-235d-4332-ab2b-50ec09cd9812 | Oracle ID: 8d6e39b0-a190-40a0-a8e1-ee82f477376f
// IMPLEMENTED — destroys a target artifact without regeneration, then its
// controller gains life equal to its mana value.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::CRUMBLE,
    oracle_id = "8d6e39b0-a190-40a0-a8e1-ee82f477376f",
    scryfall_id = "4283c407-235d-4332-ab2b-50ec09cd9812",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Crumble",
        mana_cost = mana!("{G}"),
        types = TypeSet::INSTANT,
    ),],
    abilities = &[spell!(
        &[
            Effect::destroy_no_regen(TargetSpec::Object(&Filter::ARTIFACT)),
            Effect::GainLifeFor {
                amount: Amount::TargetCmc,
                who: PlayerRel::ControllerOfTarget,
            },
        ],
        targets = Some(TargetReq::one(TargetSpec::Object(&Filter::ARTIFACT)))
    )],
);
