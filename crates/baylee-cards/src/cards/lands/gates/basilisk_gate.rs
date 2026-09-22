//! Basilisk Gate — (no cost) — Land — Gate
//! Oracle: {T}: Add {C}.
//! Oracle: {2}, {T}: Target creature gets +X/+X until end of turn, where X is the number of Gates you control. Activate only as a sorcery.
//! Set: CLB #346 — Commander Legends: Battle for Baldur's Gate | Scryfall ID: 4a306025-d429-4006-b7ed-bdb287e83f57 | Oracle ID: 8733a4fc-4068-4af4-9598-dc3d895e8556
// IMPLEMENTED — {T}: Add {C}, plus a sorcery-speed {2}, {T} pump whose size is
// the number of Gates you control, counted as the ability resolves.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

/// "A Gate you control" — the card's own subtype, so the source counts
/// itself, which is what "the number of Gates you control" means.
static GATES_YOU_CONTROL: Filter = Filter::And(&[
    Filter::HasSubtype(subtypes::land::GATE),
    Filter::ControlledByYou,
]);

card!(
    index = index::BASILISK_GATE,
    oracle_id = "8733a4fc-4068-4af4-9598-dc3d895e8556",
    scryfall_id = "4a306025-d429-4006-b7ed-bdb287e83f57",
    faces = &[face!(
        name = "Basilisk Gate",
        types = TypeSet::LAND,
        subtypes = &[subtypes::land::GATE],
    ),],
    coverage = Coverage::Implemented,
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        activated!(
            cost!("{2}", TapSelf),
            &[Effect::PumpTarget {
                power: Amount::CountOf {
                    filter: &GATES_YOU_CONTROL,
                    zone: ZoneSel::Battlefield,
                },
                toughness: Amount::CountOf {
                    filter: &GATES_YOU_CONTROL,
                    zone: ZoneSel::Battlefield,
                },
                keywords: KeywordSet::EMPTY,
                duration: Duration::UntilEndOfTurn,
            }],
            target = Some(TargetSpec::Object(&Filter::CREATURE)),
            timing = ActivationTiming::SorcerySpeed,
        ),
    ],
);
