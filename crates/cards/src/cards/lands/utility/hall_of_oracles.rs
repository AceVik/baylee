//! Hall of Oracles — (no cost) — Land
//! Oracle: {T}: Add {C}.
//! Oracle: {1}, {T}: Add one mana of any color.
//! Oracle: {T}: Put a +1/+1 counter on target creature. Activate only as a sorcery and only if you've cast an instant or sorcery spell this turn.
//! Set: SOC #378 — Secrets of Strixhaven Commander | Scryfall ID: c29b896e-eee0-4f40-89b8-89262dae44e1 | Oracle ID: 0bb896ba-e15e-43a9-9120-e674d7ba003c

// IMPLEMENTED — all printed abilities; exercised in engine card tests.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::HALL_OF_ORACLES,
    oracle_id = "0bb896ba-e15e-43a9-9120-e674d7ba003c",
    scryfall_id = "c29b896e-eee0-4f40-89b8-89262dae44e1",
    faces = &[face!(name = "Hall of Oracles", types = TypeSet::LAND,),],
    coverage = Coverage::Implemented,
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        mana_ability!(cost!("{1}", TapSelf), &[Effect::mana_of_any_color()]),
        activated!(
            cost!(TapSelf),
            &[Effect::AddCounter {
                kind: CounterKind::P1P1,
                amount: Amount::Fixed(1)
            }],
            target = Some(TargetSpec::Object(&Filter::CREATURE)),
            timing = ActivationTiming::SorcerySpeed,
            condition = Some(Condition::CastInstantOrSorceryThisTurn)
        ),
    ],
);
