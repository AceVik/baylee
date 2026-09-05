//! Basilisk Gate — (no cost) — Land — Gate
//! Oracle: {T}: Add {C}.
//! Oracle: {2}, {T}: Target creature gets +X/+X until end of turn, where X is the number of Gates you control. Activate only as a sorcery.
//! Set: CLB #346 — Commander Legends: Battle for Baldur's Gate | Scryfall ID: 4a306025-d429-4006-b7ed-bdb287e83f57 | Oracle ID: 8733a4fc-4068-4af4-9598-dc3d895e8556
// IMPLEMENTED — tap for {C}; {2},{T} at sorcery speed to give target creature
// +X/+X until EOT where X = Gates you control.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

// "the number of Gates you control" — includes Basilisk Gate itself
static GATES_YOU_CONTROL: Filter = Filter::And(&[
    Filter::ControlledByYou,
    Filter::HasSubtype(subtypes::land::GATE),
]);

static TARGET_CREATURE: Filter = Filter::CREATURE;

card! {
    index: 264,
    oracle_id: "8733a4fc-4068-4af4-9598-dc3d895e8556",
    scryfall_id: "4a306025-d429-4006-b7ed-bdb287e83f57",
    faces: &[
    face! {
        name: "Basilisk Gate",
        types: TypeSet::LAND,
        subtypes: &[subtypes::land::GATE],
    },
    ],
    coverage: Coverage::Implemented,
    abilities: &[
        // {T}: Add {C}.
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        // {2}, {T}: Target creature gets +X/+X until end of turn,
        // where X is the number of Gates you control.
        // Activate only as a sorcery.
        activated!(
            Cost {
                mana: baylee_core::mana!("{2}"),
                parts: &[CostPart::TapSelf],
            },
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
            target: Some(TargetSpec::Object(&TARGET_CREATURE)),
            timing: ActivationTiming::SorcerySpeed
        ),
    ],
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn card_data() {
        assert_eq!(CARD.index.get(), 264);
        assert_eq!(CARD.oracle_id, "8733a4fc-4068-4af4-9598-dc3d895e8556");
        assert_eq!(CARD.scryfall_id, "4a306025-d429-4006-b7ed-bdb287e83f57");
        assert_eq!(CARD.faces[0].name, "Basilisk Gate");
        assert_eq!(CARD.faces[0].types, TypeSet::LAND);
        assert_eq!(CARD.faces[0].subtypes, &[subtypes::land::GATE]);
        assert_eq!(CARD.coverage, Coverage::Implemented);
        assert_eq!(CARD.abilities.len(), 2);
    }
}

// Engine-level tests (baylee-engine): basilisk_gate_pump_scales_with_gates —
// activate with N Gates in play (including Basilisk Gate itself) and verify
// the target creature receives +N/+N until end of turn; confirm sorcery-speed
// restriction prevents activation at instant speed.
