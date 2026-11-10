//! Clergy of the Holy Nimbus — {W} — Creature — Human Cleric
//! Oracle: If this creature would be destroyed, regenerate it.
//! Oracle: {1}: This creature can't be regenerated this turn. Only your opponents may activate this ability.
//! Set: LEG #6 — Legends | Scryfall ID: db1f578f-fa3b-4447-953b-1490852b6c80 | Oracle ID: 66566999-f70a-4f14-9bf0-23325295a977
use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::CLERGY_OF_THE_HOLY_NIMBUS,
    oracle_id = "66566999-f70a-4f14-9bf0-23325295a977",
    scryfall_id = "db1f578f-fa3b-4447-953b-1490852b6c80",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Clergy of the Holy Nimbus",
        mana_cost = mana!("{W}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::CLERIC],
        power = Some(1),
        toughness = Some(1)
    )],
    coverage = Coverage::Implemented,
    abilities = &[
        static_ability!(Filter::This, Modifier::RegenerateInsteadOfDestroy),
        activated!(
            cost!("{1}"),
            &[Effect::CantBeRegeneratedThisTurn {
                target: TargetSpec::ThisObject
            }],
            zone = ActivationZone::BattlefieldOpponent
        ),
    ],
);
