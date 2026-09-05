//! Baldur's Gate — (no cost) — Legendary Land — Gate
//! Oracle: {T}: Add {C}.
//! Oracle: {2}, {T}: Add X mana of any one color, where X is the number of other Gates you control.
//! Set: CLB #345 — Commander Legends: Battle for Baldur's Gate | Scryfall ID: 2436aa14-9200-4295-8041-b682cf3c4216 | Oracle ID: da307ea2-4df7-4d6b-be0f-9dc6ac93db61
// IMPLEMENTED — tap for {C} or {2}, {T} for X mana of any one color (X = other Gates you control).

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static OTHER_GATES: Filter = Filter::And(&[
    Filter::ControlledByYou,
    Filter::Another,
    Filter::HasSubtype(subtypes::land::GATE),
]);

card! {
    index: 254,
    oracle_id: "da307ea2-4df7-4d6b-be0f-9dc6ac93db61",
    scryfall_id: "2436aa14-9200-4295-8041-b682cf3c4216",
    faces: &[
    face! {
        name: "Baldur's Gate",
        types: TypeSet::LAND,
        supertypes: SupertypeSet::LEGENDARY,
        subtypes: &[subtypes::land::GATE],
    },
    ],
    coverage: Coverage::Implemented,
    abilities: &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        mana_ability!(
            Cost {
                mana: baylee_core::mana!("{2}"),
                parts: &[CostPart::TapSelf],
            },
            &[Effect::AddMana {
                source: ManaSource::Choice(ALL_MANA_COLORS),
                amount: Amount::CountOf {
                    filter: &OTHER_GATES,
                    zone: ZoneSel::Battlefield,
                },
                combination: false,
                restriction: None,
            }],
        ),
    ],
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn card_data() {
        assert_eq!(CARD.index.get(), 254);
        assert_eq!(CARD.oracle_id, "da307ea2-4df7-4d6b-be0f-9dc6ac93db61");
        assert_eq!(CARD.scryfall_id, "2436aa14-9200-4295-8041-b682cf3c4216");
        assert_eq!(CARD.faces[0].name, "Baldur's Gate");
        assert_eq!(CARD.faces[0].types, TypeSet::LAND);
        assert_eq!(CARD.faces[0].supertypes, SupertypeSet::LEGENDARY);
        assert_eq!(CARD.faces[0].subtypes, &[subtypes::land::GATE]);
        assert_eq!(CARD.coverage, Coverage::Implemented);
        assert_eq!(CARD.abilities.len(), 2);
    }
}
