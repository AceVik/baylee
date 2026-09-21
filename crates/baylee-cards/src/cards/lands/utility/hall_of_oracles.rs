//! Hall of Oracles — (no cost) — Land
//! Oracle: {T}: Add {C}.
//! Oracle: {1}, {T}: Add one mana of any color.
//! Oracle: {T}: Put a +1/+1 counter on target creature. Activate only as a sorcery and only if you've cast an instant or sorcery spell this turn.
//! Set: SOC #378 — Secrets of Strixhaven Commander | Scryfall ID: c29b896e-eee0-4f40-89b8-89262dae44e1 | Oracle ID: 0bb896ba-e15e-43a9-9120-e674d7ba003c
// PARTIAL — both mana abilities are built; the counter ability is dropped
// because its gate is not sayable (see the NOT SUPPORTED line below).

use baylee_cards_dsl::prelude::*;

card!(
    index = index::HALL_OF_ORACLES,
    oracle_id = "0bb896ba-e15e-43a9-9120-e674d7ba003c",
    scryfall_id = "c29b896e-eee0-4f40-89b8-89262dae44e1",
    faces = &[face!(name = "Hall of Oracles", types = TypeSet::LAND,),],
    coverage = Coverage::Partial(
        "the \"only if you've cast an instant or sorcery spell this turn\" gate has no Condition variant",
    ),
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        mana_ability!(cost!("{1}", TapSelf), &[Effect::mana_of_any_color()]),
        // NOT SUPPORTED: "{T}: Put a +1/+1 counter on target creature. Activate only as a sorcery and only if you've cast an instant or sorcery spell this turn." — `Condition` has no variant for "you've cast an instant or sorcery spell this turn", and an ability that drops a printed activation gate is offered on every board, so the ability comes off the card rather than shipping ungated.
    ],
);
