//! Thalakos Deceiver — {3}{U} — Creature — Thalakos Wizard
//! Oracle: Shadow (This creature can block or be blocked by only creatures with shadow.)
//! Oracle: Whenever this creature attacks and isn't blocked, you may sacrifice it. If you do, gain control of target creature. (This effect lasts indefinitely.)
//! Set: STH #45 — Stronghold | Scryfall ID: ae4cc2f5-3a50-40c8-87ea-6b92f081f55c | Oracle ID: c304777a-39a9-4a2a-8cec-8116afb34c5b
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::THALAKOS_DECEIVER,
    oracle_id = "c304777a-39a9-4a2a-8cec-8116afb34c5b",
    scryfall_id = "ae4cc2f5-3a50-40c8-87ea-6b92f081f55c",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Implemented,
    keywords = KeywordSet::SHADOW,
    faces = &[face!(
        name = "Thalakos Deceiver",
        mana_cost = mana!("{3}{U}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::THALAKOS, subtypes::creature::WIZARD],
        power = Some(1),
        toughness = Some(1),
    ),],
    abilities = &[triggered!(
        Trigger::AttacksAndIsntBlocked(&Filter::This),
        &[Effect::MayDo {
            effects: &[Effect::SacrificeSelfThen {
                then: &[Effect::ModifyTargets {
                    modifier: Modifier::GainControl,
                    duration: Duration::Indefinitely,
                }],
            }],
        }],
        targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
    ),],
);
