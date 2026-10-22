//! Rock Slide — {X}{R} — Instant
//! Oracle: Rock Slide deals X damage divided as you choose among any number of target attacking or blocking creatures without flying.
//! Set: VIS #92 — Visions | Scryfall ID: 7e01717a-d6ed-42c1-9a9a-f3f4a3d73bca | Oracle ID: 0b2e7c1a-dc07-4138-a560-dc80fa723860
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;

static ATTACKING_OR_BLOCKING_WITHOUT_FLYING: Filter = Filter::And(&[
    Filter::CREATURE,
    Filter::Or(&[Filter::Attacking, Filter::Blocking]),
    Filter::Not(&Filter::HasKeyword(KeywordSet::FLYING)),
]);

card!(
    index = index::ROCK_SLIDE,
    oracle_id = "0b2e7c1a-dc07-4138-a560-dc80fa723860",
    scryfall_id = "7e01717a-d6ed-42c1-9a9a-f3f4a3d73bca",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Rock Slide",
        mana_cost = mana!("{X}{R}"),
        types = TypeSet::INSTANT,
    ),],
    abilities = &[spell!(
        &[Effect::DealDamageDivided {
            amount: DivisionAmount::XDoubleFrom(u32::MAX),
        }],
        targets = Some(TargetReq::up_to(
            TargetSpec::Object(&ATTACKING_OR_BLOCKING_WITHOUT_FLYING),
            u8::MAX,
        )),
    )],
);
