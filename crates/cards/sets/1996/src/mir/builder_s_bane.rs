//! Builder's Bane — {X}{X}{R} — Sorcery
//! Oracle: Destroy X target artifacts. Builder's Bane deals damage to each player equal to the number of artifacts they controlled that were put into a graveyard this way.
//! Set: MIR #160 — Mirage | Scryfall ID: fb398027-5a29-4c81-aab5-b1a2b82fd655 | Oracle ID: 814adb99-6100-4c36-96d9-763bc20dca5d
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;

card!(
    index = index::BUILDER_S_BANE,
    oracle_id = "814adb99-6100-4c36-96d9-763bc20dca5d",
    scryfall_id = "fb398027-5a29-4c81-aab5-b1a2b82fd655",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Builder's Bane",
        mana_cost = mana!("{X}{X}{R}"),
        types = TypeSet::SORCERY,
    ),],
    abilities = &[spell!(
        &[
            Effect::destroy(TargetSpec::Object(&Filter::ARTIFACT)),
            Effect::DealDamage {
                target: TargetSpec::Player(PlayerRel::EachPlayer),
                amount: Amount::ThisWayBy {
                    what: ThisWay::Died(&Filter::ARTIFACT),
                    who: PlayerRel::Recipient,
                },
            },
        ],
        targets = Some(TargetReq::x_targets(TargetSpec::Object(&Filter::ARTIFACT))),
    )],
);
