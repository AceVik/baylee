//! Downhill Charge — {2}{R} — Instant
//! Oracle: You may sacrifice a Mountain rather than pay this spell's mana cost.
//! Oracle: Target creature gets +X/+0 until end of turn, where X is the number of Mountains you control.
//! Set: DDI #69 — Duel Decks: Venser vs. Koth | Scryfall ID: 8d7ef860-f248-4b14-8080-592121983d83 | Oracle ID: 8018419c-ea19-499b-a33a-7c20b35bce7b
// NOT SUPPORTED: target creature gets +X/+0 until end of turn for each Mountain you control when cast for {2}{R}; the alternative cost of sacrificing a Mountain is not yet modeled
// PARTIAL: target creature gets +X/+0 until end of turn for each Mountain you control when cast for {2}{R}; the alternative cost of sacrificing a Mountain is not yet modeled.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static MOUNTAIN_YOU_CONTROL: Filter = Filter::And(&[
    Filter::HasSubtype(subtypes::land::MOUNTAIN),
    Filter::ControlledByYou,
]);

card!(
    index = index::DOWNHILL_CHARGE,
    oracle_id = "8018419c-ea19-499b-a33a-7c20b35bce7b",
    scryfall_id = "8d7ef860-f248-4b14-8080-592121983d83",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Partial(
        "target creature gets +X/+0 until end of turn for each Mountain you control when cast for {2}{R}; the alternative cost of sacrificing a Mountain is not yet modeled",
    ),
    faces = &[face!(
        name = "Downhill Charge",
        mana_cost = mana!("{2}{R}"),
        types = TypeSet::INSTANT,
    ),],
    abilities = &[spell!(
        &[Effect::PumpTarget {
            power: Amount::CountOf {
                filter: &MOUNTAIN_YOU_CONTROL,
                zone: ZoneSel::Battlefield,
            },
            toughness: Amount::Fixed(0),
            keywords: KeywordSet::EMPTY,
            duration: Duration::UntilEndOfTurn,
        }],
        targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
    ),],
);
