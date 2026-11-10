//! Centaur Garden — (no cost) — Land
//! Oracle: {T}: Add {G}. This land deals 1 damage to you.
//! Oracle: Threshold — {G}, {T}, Sacrifice this land: Target creature gets +3/+3 until end of turn. Activate only if there are seven or more cards in your graveyard.
//! Set: ODY #316 — Odyssey | Scryfall ID: ca041d5e-c65f-4e7e-ae4f-9c748a069aa3 | Oracle ID: 5cf92fd4-7c0b-4d8e-92f1-53dc2e0476fc
// IMPLEMENTED — the pain mana line, and the +3/+3 gated on threshold.
// The gate was missing and the ability shipped without it, which is a
// land strictly stronger than the printed one.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::CENTAUR_GARDEN,
    oracle_id = "5cf92fd4-7c0b-4d8e-92f1-53dc2e0476fc",
    scryfall_id = "ca041d5e-c65f-4e7e-ae4f-9c748a069aa3",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(name = "Centaur Garden", types = TypeSet::LAND,),],
    coverage = Coverage::Implemented,
    abilities = &[
        mana_ability!(&[
            Effect::mana(ManaColor::Green, 1),
            Effect::DealDamage {
                amount: Amount::Fixed(1),
                target: TargetSpec::Player(PlayerRel::You),
            },
        ]),
        activated!(
            cost!("{G}", TapSelf, SacrificeSelf),
            &[Effect::PumpTarget {
                power: Amount::Fixed(3),
                toughness: Amount::Fixed(3),
                keywords: KeywordSet::EMPTY,
                duration: Duration::UntilEndOfTurn,
            }],
            target = Some(TargetSpec::Object(&Filter::CREATURE)),
            condition = Some(Condition::GraveyardCountAtLeast(7)),
        ),
    ],
);
