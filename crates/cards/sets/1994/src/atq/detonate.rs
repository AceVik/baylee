//! Detonate — {X}{R} — Sorcery
//! Oracle: Destroy target artifact with mana value X. It can't be regenerated. Detonate deals X damage to that artifact's controller.
//! Set: ME4 #111 — Masters Edition IV | Scryfall ID: 45d14cd5-8398-481a-a190-d6c0f6263e33 | Oracle ID: daa90a75-c600-41bd-9311-ec21cf51480b
// IMPLEMENTED — destroys a target artifact whose mana value equals the
// announced X, then deals X damage to that artifact's controller.

use baylee_cards_dsl::prelude::*;

/// "Target artifact with mana value X", X being the value announced for this
/// spell (CR 107.3a).
static ARTIFACT_WITH_MANA_VALUE_X: Filter = Filter::And(&[Filter::ARTIFACT, Filter::CmcExactlyX]);

card!(
    index = index::DETONATE,
    oracle_id = "daa90a75-c600-41bd-9311-ec21cf51480b",
    scryfall_id = "45d14cd5-8398-481a-a190-d6c0f6263e33",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Detonate",
        mana_cost = mana!("{X}{R}"),
        types = TypeSet::SORCERY,
    ),],
    abilities = &[spell!(
        &[
            Effect::destroy_no_regen(TargetSpec::Object(&ARTIFACT_WITH_MANA_VALUE_X)),
            Effect::DealDamageToTargetController { amount: Amount::X },
        ],
        targets = Some(TargetReq::one(TargetSpec::Object(
            &ARTIFACT_WITH_MANA_VALUE_X
        )))
    )],
);
