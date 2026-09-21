//! Centaur Garden — (no cost) — Land
//! Oracle: {T}: Add {G}. This land deals 1 damage to you.
//! Oracle: Threshold — {G}, {T}, Sacrifice this land: Target creature gets +3/+3 until end of turn. Activate only if there are seven or more cards in your graveyard.
//! Set: ODY #316 — Odyssey | Scryfall ID: ca041d5e-c65f-4e7e-ae4f-9c748a069aa3 | Oracle ID: 5cf92fd4-7c0b-4d8e-92f1-53dc2e0476fc
// PARTIAL — the mana ability ({G} plus the 1 damage to you) and the threshold
// pump are built; the graveyard gate has no Condition variant.

use baylee_cards_dsl::prelude::*;

// NOT SUPPORTED: "Activate only if there are seven or more cards in your
// graveyard" — `Condition` has `OpponentGraveyardCountAtLeast` and no
// your-graveyard counterpart, so the pump ability is offered unconditionally
// (the handling docs/card-dsl.md records for an unenforceable activation
// condition).

card!(
    index = index::CENTAUR_GARDEN,
    oracle_id = "5cf92fd4-7c0b-4d8e-92f1-53dc2e0476fc",
    scryfall_id = "ca041d5e-c65f-4e7e-ae4f-9c748a069aa3",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(name = "Centaur Garden", types = TypeSet::LAND,),],
    coverage = Coverage::Partial(
        "the threshold gate (seven or more cards in your graveyard) has no \
         Condition variant, so the pump is offered unconditionally"
    ),
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
            target = Some(TargetSpec::Object(&Filter::CREATURE))
        ),
    ],
);
