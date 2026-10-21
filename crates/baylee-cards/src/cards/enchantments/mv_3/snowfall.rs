//! Snowfall — {2}{U} — Enchantment
//! Oracle: Cumulative upkeep {U} (At the beginning of your upkeep, put an age counter on this permanent, then sacrifice it unless you pay its upkeep cost for each age counter on it.)
//! Oracle: Whenever an Island is tapped for mana, its controller may add an additional {U}. If that Island is snow, its controller may add an additional {U}{U} instead. Spend this mana only to pay cumulative upkeep costs.
//! Set: ICE #101 — Ice Age | Scryfall ID: 788ed793-3993-4a63-b9f9-9ac3947c3108 | Oracle ID: 5ffed544-1656-4753-ac8f-ad8fb4442c7f

use baylee_cards_dsl::prelude::*;

card!(
    index = index::SNOWFALL,
    oracle_id = "5ffed544-1656-4753-ac8f-ad8fb4442c7f",
    scryfall_id = "788ed793-3993-4a63-b9f9-9ac3947c3108",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Partial(
        "cumulative upkeep {U} works; the triggered ability when an Island is tapped for mana to let its controller add {U} (or {U}{U} if snow) restricted to cumulative upkeep costs is not supported",
    ),
    faces = &[face!(
        name = "Snowfall",
        mana_cost = mana!("{2}{U}"),
        types = TypeSet::ENCHANTMENT,
    ),],
    abilities = &[
        // CR 702.24a: Cumulative upkeep {U}.
        triggered!(
            Trigger::StepBegin {
                step: StepKind::Upkeep,
                whose: PlayerRel::You,
            },
            &[
                Effect::AddCounter {
                    kind: counters::AGE,
                    amount: Amount::Fixed(1),
                },
                Effect::PlayerMayPayManaEachThenElse {
                    player: PlayerRel::You,
                    each: mana!("{U}"),
                    times: Amount::CountersOnSource(counters::AGE),
                    paid: &[],
                    unpaid: &[Effect::SacrificeSelf],
                },
            ],
            condition = Some(Condition::SourceMatches(&Filter::InZone(
                ZoneRef::Battlefield,
            ))),
        ),
        // NOT SUPPORTED: Triggered mana ability allowing an Island's controller to add {U} (or {U}{U} if snow) restricted to cumulative upkeep costs (AddManaFor lacks SpendRestriction support).
    ],
);
