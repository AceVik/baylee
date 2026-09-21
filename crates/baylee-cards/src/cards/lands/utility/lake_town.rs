//! Lake-town — (no cost) — Land
//! Oracle: This land enters tapped.
//! Oracle: {T}: Add {W} or {U}.
//! Oracle: {2}{W}{U}, {T}, Sacrifice this land: Put two +1/+1 counters on target Human you control. Activate only as a sorcery.
//! Set: HOB #186 — The Hobbit | Scryfall ID: 2fbd0584-81a7-4c47-8af1-1c8635899a97 | Oracle ID: 717c6beb-81c6-43ed-aab0-aedfc1cbac33
// IMPLEMENTED — enters tapped, {T} for {W} or {U}, and {2}{W}{U}, {T}, sac puts two +1/+1 counters on target Human you control at sorcery speed.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes::creature;

card!(
    index = index::LAKE_TOWN,
    oracle_id = "717c6beb-81c6-43ed-aab0-aedfc1cbac33",
    scryfall_id = "2fbd0584-81a7-4c47-8af1-1c8635899a97",
    color_identity = ColorSet::from_slice(&[Color::Blue, Color::White]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Lake-town",
        types = TypeSet::LAND,
        enter_modifiers = &[EnterModifier::Tapped],
    ),],
    abilities = &[
        mana_ability!(&[Effect::mana_choice(&[ManaColor::White, ManaColor::Blue])]),
        activated!(
            cost!("{2}{W}{U}", TapSelf, SacrificeSelf),
            &[Effect::AddCounter {
                kind: CounterKind::P1P1,
                amount: Amount::Fixed(2),
            }],
            target = Some(TargetSpec::Object(
                &f!(your Filter::HasSubtype(creature::HUMAN))
            )),
            timing = ActivationTiming::SorcerySpeed,
        ),
    ],
);
