//! Polar Kraken — {8}{U}{U}{U} — Creature — Kraken
//! Oracle: Trample
//! Oracle: This creature enters tapped.
//! Oracle: Cumulative upkeep—Sacrifice a land. (At the beginning of your upkeep, put an age counter on this permanent, then sacrifice it unless you pay its upkeep cost for each age counter on it.)
//! Set: ME1 #44 — Masters Edition | Scryfall ID: c4643512-6071-45c4-bffc-8feec5bb094f | Oracle ID: d0ff30c8-ddb7-439d-b14d-c83fe3c8da87
use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

// NOT SUPPORTED: No Effect repeats a non-mana, non-life upkeep cost per age counter, so Polar Kraken puts an age counter on itself each upkeep and requires sacrificing a single land rather than one land per age counter.
card!(
    index = index::POLAR_KRAKEN,
    oracle_id = "d0ff30c8-ddb7-439d-b14d-c83fe3c8da87",
    scryfall_id = "c4643512-6071-45c4-bffc-8feec5bb094f",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    keywords = KeywordSet::TRAMPLE,
    coverage = Coverage::Partial(
        "cumulative upkeep sacrifices one land each upkeep instead of one land per age counter on Polar Kraken",
    ),
    faces = &[face!(
        name = "Polar Kraken",
        mana_cost = mana!("{8}{U}{U}{U}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::KRAKEN],
        power = Some(11),
        toughness = Some(11),
        enter_modifiers = &[EnterModifier::Tapped],
    ),],
    abilities = &[triggered!(
        Trigger::StepBegin {
            step: StepKind::Upkeep,
            whose: PlayerRel::You,
        },
        &[
            Effect::AddCounter {
                kind: counters::AGE,
                amount: Amount::Fixed(1),
            },
            Effect::PlayerMayPayCostOr {
                player: PlayerRel::You,
                cost: &CostPart::Sacrifice(&Filter::LAND),
                effect: &Effect::SacrificeSelf,
            },
        ],
        condition = Some(Condition::SourceMatches(&Filter::InZone(
            ZoneRef::Battlefield,
        ))),
    )],
);
