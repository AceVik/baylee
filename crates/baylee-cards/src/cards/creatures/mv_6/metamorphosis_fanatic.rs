//! Metamorphosis Fanatic — {4}{B}{B} — Creature — Human Cleric
//! Oracle: Lifelink
//! Oracle: When this creature enters, return up to one target creature card from your graveyard to the battlefield with a lifelink counter on it.
//! Oracle: Miracle {1}{B} (You may cast this card for its miracle cost when you draw it if it's the first card you drew this turn.)
//! Set: DSC #21 — Duskmourn: House of Horror Commander | Scryfall ID: 16448d95-ee21-4def-b880-26f6f159c213 | Oracle ID: 017aa9b3-a8ea-4588-9c50-e914a7d8e4ee
// IMPLEMENTED — lifelink 4/4 + ETB reanimate with a lifelink counter +
// miracle cast.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes::creature;

card! {
    index: 94,
    oracle_id: "017aa9b3-a8ea-4588-9c50-e914a7d8e4ee",
    scryfall_id: "16448d95-ee21-4def-b880-26f6f159c213",
    faces: &[face! {
        name: "Metamorphosis Fanatic",
        mana_cost: baylee_core::mana!("{4}{B}{B}"),
        types: TypeSet::CREATURE,
        subtypes: &[creature::HUMAN, creature::CLERIC],
        power: Some(4),
        toughness: Some(4),
        miracle: Some(baylee_core::mana!("{1}{B}")),
    }],
    color_identity: ColorSet::from_slice(&[Color::Black]),
    keywords: KeywordSet::LIFELINK,
    coverage: Coverage::Implemented,
    abilities: &[triggered!(Trigger::EntersBattlefield(&Filter::This), &[
            Effect::GraveyardToBattlefield {
                target: TargetSpec::CardInGraveyard(&Filter::CREATURE, PlayerRel::You),
            },
            Effect::AddCounter {
                kind: CounterKind::Lifelink,
                amount: Amount::Fixed(1),
            },
        ], targets: Some(TargetReq {
            spec: TargetSpec::CardInGraveyard(&Filter::CREATURE, PlayerRel::You),
            min: 0,
            max: 1,
            count_is_x: false,
        }))],
}
