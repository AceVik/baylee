//! Syphon Soul — {2}{B} — Sorcery
//! Oracle: Syphon Soul deals 2 damage to each other player. You gain life equal to the damage dealt this way.
//! Set: CNS #127 — Conspiracy | Scryfall ID: dc308882-1c12-4da0-8eae-caf4cc3d43e1 | Oracle ID: d47e55a6-7a16-4ed9-a94b-f1575588ee9e
// PARTIAL — the 2 damage to each opponent is built; the life gain is off the
// card because no amount reads the damage the spell just dealt.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::SYPHON_SOUL,
    oracle_id = "d47e55a6-7a16-4ed9-a94b-f1575588ee9e",
    scryfall_id = "dc308882-1c12-4da0-8eae-caf4cc3d43e1",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Syphon Soul",
        mana_cost = mana!("{2}{B}"),
        types = TypeSet::SORCERY,
    ),],
    coverage = Coverage::Partial(
        "the life gain is not written: no Amount reads the damage dealt by the \
         resolving spell, so its amount cannot be tied to what was actually dealt",
    ),
    // NOT SUPPORTED: "You gain life equal to the damage dealt this way." — the
    // damage's own amount is written, but no `Amount` reads what the resolving
    // effect just dealt: `Amount::EventAmount` belongs to a triggered ability,
    // and `Amount::DamageDealtToYouThisTurn` counts damage dealt to the
    // controller, the reverse of this. `Effect::GainLifeFor` with
    // `Amount::Fixed(2)` would gain 2 even where the damage was prevented.
    abilities = &[spell!(&[Effect::DealDamage {
        amount: Amount::Fixed(2),
        target: TargetSpec::Player(PlayerRel::EachOpponent),
    }])],
);
