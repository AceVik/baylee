//! Cave of Temptation — (no cost) — Land
//! Oracle: {T}: Add {C}.
//! Oracle: {1}, {T}: Add one mana of any color.
//! Oracle: {4}, {T}, Sacrifice this land: Put two +1/+1 counters on target creature. Activate only as a sorcery.
//! Set: MH1 #237 — Modern Horizons | Scryfall ID: d86e9149-6fd9-44fc-b765-3e646c7d83d6 | Oracle ID: 75540897-53f6-433b-bd70-9851551df6ef
// IMPLEMENTED — land with three activated abilities: {T} → {C}; {1}{T} → any color; {4}{T} + sacrifice self → two +1/+1 counters on target creature (sorcery speed).

use baylee_cards_dsl::prelude::*;

card! {
    index: 341,
    oracle_id: "75540897-53f6-433b-bd70-9851551df6ef",
    scryfall_id: "d86e9149-6fd9-44fc-b765-3e646c7d83d6",
    faces: &[
    face! {
        name: "Cave of Temptation",
        types: TypeSet::LAND,
    },
    ],
    coverage: Coverage::Implemented,
    abilities: &[
        // {T}: Add {C}.
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        // {1}, {T}: Add one mana of any color.
        mana_ability!(
            Cost {
                mana: baylee_core::mana!("{1}"),
                parts: &[CostPart::TapSelf],
            },
            &[Effect::mana_of_any_color()],
        ),
        // {4}, {T}, Sacrifice this land: Put two +1/+1 counters on target
        // creature. Activate only as a sorcery.
        activated!(
            Cost {
                mana: baylee_core::mana!("{4}"),
                parts: &[CostPart::TapSelf, CostPart::SacrificeSelf],
            },
            &[Effect::AddCounter {
                kind: CounterKind::P1P1,
                amount: Amount::Fixed(2),
            }],
            target: Some(TargetSpec::Object(&Filter::CREATURE)),
            timing: ActivationTiming::SorcerySpeed,
        ),
    ],
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn card_data() {
        assert_eq!(CARD.index.get(), 341);
        assert_eq!(CARD.oracle_id, "75540897-53f6-433b-bd70-9851551df6ef");
        assert_eq!(CARD.scryfall_id, "d86e9149-6fd9-44fc-b765-3e646c7d83d6");
        assert_eq!(CARD.faces[0].name, "Cave of Temptation");
        assert_eq!(CARD.faces[0].types, TypeSet::LAND);
        assert_eq!(CARD.coverage, Coverage::Implemented);
        assert_eq!(CARD.abilities.len(), 3);
    }
}
