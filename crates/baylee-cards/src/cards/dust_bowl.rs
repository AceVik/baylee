//! Dust Bowl — (no cost) — Land
//! Oracle: {T}: Add {C}.
//! Oracle: {3}, {T}, Sacrifice a land: Destroy target nonbasic land.
//! Set: MMQ #316 — Mercadian Masques | Scryfall ID: 75b03c30-c2b8-4207-b675-26c59c40a7e5 | Oracle ID: d3df7128-31dd-4d71-90be-87e2e9ff51b4
// IMPLEMENTED — tap for {C}; {3}, tap, sacrifice a land to destroy target nonbasic land.

use baylee_cards_dsl::prelude::*;

static NONBASIC_LAND: Filter = Filter::And(&[
    Filter::LAND,
    Filter::Not(&Filter::HasSupertype(SupertypeSet::BASIC)),
]);

card! {
    index: 450,
    oracle_id: "d3df7128-31dd-4d71-90be-87e2e9ff51b4",
    scryfall_id: "75b03c30-c2b8-4207-b675-26c59c40a7e5",
    coverage: Coverage::Implemented,
    faces: &[
    face! {
        name: "Dust Bowl",
        types: TypeSet::LAND,
    },
    ],
    abilities: &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        activated!(
            Cost {
                mana: baylee_core::mana!("{3}"),
                parts: &[CostPart::TapSelf, CostPart::Sacrifice(&Filter::LAND)],
            },
            &[Effect::Destroy {
                target: TargetSpec::Object(&NONBASIC_LAND),
            }],
            target: Some(TargetSpec::Object(&NONBASIC_LAND)),
        ),
    ],
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn card_data() {
        assert_eq!(CARD.index.get(), 450);
        assert_eq!(CARD.oracle_id, "d3df7128-31dd-4d71-90be-87e2e9ff51b4");
        assert_eq!(CARD.scryfall_id, "75b03c30-c2b8-4207-b675-26c59c40a7e5");
        assert_eq!(CARD.faces[0].name, "Dust Bowl");
        assert_eq!(CARD.faces[0].types, TypeSet::LAND);
        assert_eq!(CARD.coverage, Coverage::Implemented);
        assert_eq!(CARD.color_identity, ColorSet::EMPTY);
        assert_eq!(CARD.abilities.len(), 2);
    }
}

