//! Fiery Gambit — {2}{R} — Sorcery
//! Oracle: Flip a coin until you lose a flip or choose to stop flipping. If you lose a flip, Fiery Gambit has no effect. If you win one or more flips, Fiery Gambit deals 3 damage to target creature. If you win two or more flips, Fiery Gambit deals 6 damage to each opponent. If you win three or more flips, draw nine cards and untap all lands you control.
//! Set: MRD #90 — Mirrodin | Scryfall ID: a91376ed-5868-4887-8389-5ef5b9471786 | Oracle ID: 78497c54-7179-41b7-9454-7b915fd96cd5
use baylee_cards_dsl::prelude::*;

card!(
    index = index::FIERY_GAMBIT,
    oracle_id = "78497c54-7179-41b7-9454-7b915fd96cd5",
    scryfall_id = "a91376ed-5868-4887-8389-5ef5b9471786",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Fiery Gambit",
        mana_cost = mana!("{2}{R}"),
        types = TypeSet::SORCERY,
    ),],
    coverage = Coverage::Implemented,
    abilities = &[spell!(
        &[Effect::FlipCoinsUntilLossOrStopCumulative],
        targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
    )],
);
