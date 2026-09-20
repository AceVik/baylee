//! Kessig Wolf Run — (no cost) — Land
//! Oracle: {T}: Add {C}.
//! Oracle: {X}{R}{G}, {T}: Target creature gets +X/+0 and gains trample until end of turn.
//! Set: TDC #375 — Tarkir: Dragonstorm Commander | Scryfall ID: 32da0d6c-64dd-4aec-b63e-953e96164603 | Oracle ID: c6911265-54ef-4c16-bcf2-1ffb24b7d426
// IMPLEMENTED — {T}: Add {C}, and the {X}{R}{G}, {T} pump: +X/+0 (Amount::X)
// plus trample (KeywordSet::TRAMPLE) until end of turn on one target creature.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::KESSIG_WOLF_RUN,
    oracle_id = "c6911265-54ef-4c16-bcf2-1ffb24b7d426",
    scryfall_id = "32da0d6c-64dd-4aec-b63e-953e96164603",
    color_identity = ColorSet::from_slice(&[Color::Green, Color::Red]),
    coverage = Coverage::Implemented,
    faces = &[face!(name = "Kessig Wolf Run", types = TypeSet::LAND,),],
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        activated!(
            cost!("{X}{R}{G}", TapSelf),
            &[Effect::PumpTarget {
                power: Amount::X,
                toughness: Amount::Fixed(0),
                keywords: KeywordSet::TRAMPLE,
                duration: Duration::UntilEndOfTurn,
            }],
            target = Some(TargetSpec::Object(&Filter::CREATURE)),
        ),
    ],
);
