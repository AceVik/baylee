//! Starke of Rath — {1}{R}{R} — Legendary Creature — Human Rogue
//! Oracle: {T}: Destroy target artifact or creature. That permanent's controller gains control of Starke. (This effect lasts indefinitely.)
//! Set: TPR #162 — Tempest Remastered | Scryfall ID: a0f063fc-f1c4-4f3c-a72b-5f446d0c6381 | Oracle ID: 07bc407e-7ba1-45aa-a562-0597ebe56044

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static ARTIFACT_OR_CREATURE: Filter = Filter::HasType(TypeSet::ARTIFACT.union(TypeSet::CREATURE));

card!(
    index = index::STARKE_OF_RATH,
    oracle_id = "07bc407e-7ba1-45aa-a562-0597ebe56044",
    scryfall_id = "a0f063fc-f1c4-4f3c-a72b-5f446d0c6381",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    commander = CommanderRule::Legendary,
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Starke of Rath",
        mana_cost = mana!("{1}{R}{R}"),
        types = TypeSet::CREATURE,
        supertypes = SupertypeSet::LEGENDARY,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::ROGUE],
        power = Some(2),
        toughness = Some(2),
    ),],
    abilities = &[activated!(
        Cost::TAP,
        &[
            Effect::destroy(TargetSpec::Object(&ARTIFACT_OR_CREATURE)),
            Effect::GainControlAll {
                filter: &Filter::This,
                to: PlayerRel::ControllerOfTarget,
            },
        ],
        target = Some(TargetSpec::Object(&ARTIFACT_OR_CREATURE)),
    )],
);
