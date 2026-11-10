//! Kjeldoran Guard — {1}{W} — Creature — Human Soldier
//! Oracle: {T}: Target creature gets +1/+1 until end of turn. When that creature leaves the battlefield this turn, sacrifice this creature. Activate only during combat and only if defending player controls no snow lands.
//! Set: ICE #35 — Ice Age | Scryfall ID: bdf41f17-8f82-4a8c-adec-0f3804faff3b | Oracle ID: bd699cf8-56c7-4375-920e-8dd22c83a82c
use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static SNOW_LAND: Filter = Filter::And(&[Filter::LAND, Filter::HasSupertype(SupertypeSet::SNOW)]);

card!(
    index = index::KJELDORAN_GUARD,
    oracle_id = "bd699cf8-56c7-4375-920e-8dd22c83a82c",
    scryfall_id = "bdf41f17-8f82-4a8c-adec-0f3804faff3b",
    color_identity = ColorSet::from_slice(&[Color::White]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Kjeldoran Guard",
        mana_cost = mana!("{1}{W}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::SOLDIER],
        power = Some(1),
        toughness = Some(1),
    ),],
    abilities = &[activated!(
        Cost::TAP,
        &[
            Effect::ModifyTargets {
                modifier: Modifier::ModifyPT(1, 1),
                duration: Duration::UntilEndOfTurn,
            },
            Effect::WhenTargetLeavesBattlefieldThisTurn {
                effects: &[Effect::SacrificeSelf],
            },
        ],
        target = Some(TargetSpec::Object(&Filter::CREATURE)),
        condition = Some(Condition::All(&[
            Condition::DuringCombat,
            Condition::SomeDefendingPlayerControlsNone(&SNOW_LAND),
        ])),
    )],
);
