//! Ether Well — {3}{U} — Instant
//! Oracle: Put target creature on top of its owner's library. If that creature is red, you may put it on the bottom of its owner's library instead.
//! Set: MIR #65 — Mirage | Scryfall ID: 0a2cf195-01bf-4076-a0c6-ca5403d84f7d | Oracle ID: a2b6faca-3321-45d2-a027-ff3a8ae61f10
// IMPLEMENTED — puts target creature on top of its owner's library, or optionally on the bottom if it is red.

use baylee_cards_dsl::prelude::*;

static RED: Filter = Filter::HasColor(ColorSet::from_slice(&[Color::Red]));
static ON_BATTLEFIELD: Filter = Filter::InZone(ZoneRef::Battlefield);

card!(
    index = index::ETHER_WELL,
    oracle_id = "a2b6faca-3321-45d2-a027-ff3a8ae61f10",
    scryfall_id = "0a2cf195-01bf-4076-a0c6-ca5403d84f7d",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Ether Well",
        mana_cost = mana!("{3}{U}"),
        types = TypeSet::INSTANT,
    ),],
    abilities = &[spell!(
        &[Effect::IfTargetMatchesElse {
            filter: &RED,
            then: &[
                Effect::MayDo {
                    effects: &[Effect::PutTargetOnBottomOfLibrary],
                },
                Effect::IfTargetMatches {
                    filter: &ON_BATTLEFIELD,
                    then: &[Effect::PutTargetOnTopOfLibrary],
                },
            ],
            otherwise: &[Effect::PutTargetOnTopOfLibrary],
        }],
        targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
    )],
);
