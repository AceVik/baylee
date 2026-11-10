//! Blizzard — {G}{G} — Enchantment
//! Oracle: Cast this spell only if you control a snow land.
//! Oracle: Cumulative upkeep {2} (At the beginning of your upkeep, put an age counter on this permanent, then sacrifice it unless you pay its upkeep cost for each age counter on it.)
//! Oracle: Creatures with flying don't untap during their controllers' untap steps.
//! Set: ICE #227 — Ice Age | Scryfall ID: c369e4f9-0f2b-446c-9e2d-d3eefab0586d | Oracle ID: 698feaaa-2fff-4809-81d1-4caad44c6c0a

use baylee_cards_dsl::prelude::*;

static SNOW_LAND: Filter = Filter::And(&[Filter::LAND, Filter::HasSupertype(SupertypeSet::SNOW)]);

static FLYING_CREATURE: Filter =
    Filter::And(&[Filter::CREATURE, Filter::HasKeyword(KeywordSet::FLYING)]);

card!(
    index = index::BLIZZARD,
    oracle_id = "698feaaa-2fff-4809-81d1-4caad44c6c0a",
    scryfall_id = "c369e4f9-0f2b-446c-9e2d-d3eefab0586d",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Blizzard",
        mana_cost = mana!("{G}{G}"),
        types = TypeSet::ENCHANTMENT,
    ),],
    abilities = &[
        spell!(
            &[],
            condition = Some(Condition::ControlCount(&SNOW_LAND, 1))
        ),
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
                    each: mana!("{2}"),
                    times: Amount::CountersOnSource(counters::AGE),
                    paid: &[],
                    unpaid: &[Effect::SacrificeSelf],
                },
            ],
            condition = Some(Condition::SourceMatches(&Filter::InZone(
                ZoneRef::Battlefield
            )))
        ),
        static_ability!(FLYING_CREATURE, Modifier::DoesNotUntap),
    ],
);
