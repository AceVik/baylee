//! Scorched Earth — {X}{R} — Sorcery
//! Oracle: As an additional cost to cast this spell, discard X land cards.
//! Oracle: Destroy X target lands.
//! Set: TMP #200 — Tempest | Scryfall ID: e6a97817-d1fd-4ba4-9ced-c2702b081523 | Oracle ID: fd1ce3e0-9c2d-48b6-9a14-4e71921cdfa4
// PARTIAL: discards one land card as an additional cost and destroys X target lands; discarding X land cards as a mandatory additional cost is not in CostPart.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::SCORCHED_EARTH,
    oracle_id = "fd1ce3e0-9c2d-48b6-9a14-4e71921cdfa4",
    scryfall_id = "e6a97817-d1fd-4ba4-9ced-c2702b081523",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Partial(
        "discards one land card as an additional cost and destroys X target lands; discarding X land cards as a mandatory additional cost is not in CostPart",
    ),
    faces = &[face!(
        name = "Scorched Earth",
        mana_cost = mana!("{X}{R}"),
        types = TypeSet::SORCERY,
        mandatory_additional_costs = &[CostPart::Discard(&Filter::LAND)],
    ),],
    abilities = &[spell!(
        &[Effect::destroy(TargetSpec::Object(&Filter::LAND))],
        targets = Some(TargetReq::x_targets(TargetSpec::Object(&Filter::LAND))),
    )],
);
