//! Reverberation — {2}{U}{U} — Instant
//! Oracle: All damage that would be dealt this turn by target sorcery spell is dealt to that spell's controller instead.
//! Set: LEG #74 — Legends | Scryfall ID: a3d1f470-058d-41b7-acaf-4f68431de9ed | Oracle ID: d3088e1d-62c9-4478-9ef7-fc3c9e5cfadb
// IMPLEMENTED — redirects all damage from the targeted sorcery spell to its
// controller for the turn, preserving the original damage source.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::REVERBERATION,
    oracle_id = "d3088e1d-62c9-4478-9ef7-fc3c9e5cfadb",
    scryfall_id = "a3d1f470-058d-41b7-acaf-4f68431de9ed",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Reverberation",
        mana_cost = mana!("{2}{U}{U}"),
        types = TypeSet::INSTANT,
    ),],
    abilities = &[spell!(
        &[Effect::RedirectTargetSourceDamageThisTurn {
            source: TargetSpec::Spell(&Filter::HasType(TypeSet::SORCERY)),
            only_to_you: false,
            to: DamageRedirectTo::DamagingSourceController,
        }],
        targets = Some(TargetReq::one(TargetSpec::Spell(&Filter::HasType(
            TypeSet::SORCERY
        )))),
    )],
);
