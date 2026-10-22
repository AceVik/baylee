//! Unerring Sling — {3} — Artifact
//! Oracle: {3}, {T}, Tap an untapped creature you control: This artifact deals damage equal to the tapped creature's power to target attacking or blocking creature with flying.
//! Set: MIR #322 — Mirage | Scryfall ID: ecf946b8-8574-406a-83fd-966ed912921f | Oracle ID: 10a52a73-157f-475e-975a-eb00c80573d4
// IMPLEMENTED — {3}, {T}, Tap an untapped creature you control: deals damage equal to the tapped creature's power to target attacking or blocking creature with flying.

use baylee_cards_dsl::prelude::*;

static ATTACKING_OR_BLOCKING_WITH_FLYING: Filter = Filter::And(&[
    Filter::CREATURE,
    Filter::Or(&[Filter::Attacking, Filter::Blocking]),
    Filter::HasKeyword(KeywordSet::FLYING),
]);

card!(
    index = index::UNERRING_SLING,
    oracle_id = "10a52a73-157f-475e-975a-eb00c80573d4",
    scryfall_id = "ecf946b8-8574-406a-83fd-966ed912921f",
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Unerring Sling",
        mana_cost = mana!("{3}"),
        types = TypeSet::ARTIFACT,
    ),],
    abilities = &[activated!(
        cost!("{3}", TapSelf, TapOther(&Filter::YOUR_CREATURE)),
        &[Effect::DealDamage {
            amount: Amount::TappedPower,
            target: TargetSpec::Object(&ATTACKING_OR_BLOCKING_WITH_FLYING),
        }],
        target = Some(TargetSpec::Object(&ATTACKING_OR_BLOCKING_WITH_FLYING)),
    )],
);
