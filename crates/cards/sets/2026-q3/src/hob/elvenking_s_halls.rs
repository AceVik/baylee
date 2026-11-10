//! Elvenking's Halls — (no cost) — Land
//! Oracle: This land enters tapped.
//! Oracle: {T}: Add {G} or {U}.
//! Oracle: {2}{G}{U}, {T}, Sacrifice this land: Put two +1/+1 counters on target Elf you control. Activate only as a sorcery.
//! Set: HOB #182 — The Hobbit | Scryfall ID: cd477096-41b1-4907-9cb3-852cb22c9ba2 | Oracle ID: a91e0154-14a9-4681-8236-04db231592a4
// IMPLEMENTED — enters tapped; {T} for {G} or {U}; and the sorcery-speed
// {2}{G}{U} activation that puts two +1/+1 counters on an Elf you control.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes::creature;

card!(
    index = index::ELVENKING_S_HALLS,
    oracle_id = "a91e0154-14a9-4681-8236-04db231592a4",
    scryfall_id = "cd477096-41b1-4907-9cb3-852cb22c9ba2",
    color_identity = ColorSet::from_slice(&[Color::Green, Color::Blue]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Elvenking's Halls",
        types = TypeSet::LAND,
        enter_modifiers = &[EnterModifier::Tapped],
    ),],
    abilities = &[
        mana_ability!(&[Effect::mana_choice(&[ManaColor::Green, ManaColor::Blue])]),
        activated!(
            cost!("{2}{G}{U}", TapSelf, SacrificeSelf),
            &[Effect::AddCounter {
                kind: CounterKind::P1P1,
                amount: Amount::Fixed(2),
            }],
            target = Some(TargetSpec::Object(
                &f!(your Filter::HasSubtype(creature::ELF))
            )),
            timing = ActivationTiming::SorcerySpeed,
        ),
    ],
);
