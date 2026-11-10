//! Littjara Mirrorlake — (no cost) — Land
//! Oracle: This land enters tapped.
//! Oracle: {T}: Add {U}.
//! Oracle: {2}{G}{G}{U}, {T}, Sacrifice this land: Create a token that's a copy of target creature you control, except it enters with an additional +1/+1 counter on it. Activate only as a sorcery.
//! Set: NCC #412 — New Capenna Commander | Scryfall ID: cf1f38de-44b3-42fa-9288-d7c1832ff75f | Oracle ID: 3b577179-10d0-43f1-ac17-9dd2d12c965d

// IMPLEMENTED — all printed abilities; exercised in engine card tests.

use baylee_cards_dsl::prelude::*;

const COPY_TARGET: TargetSpec = TargetSpec::Object(&Filter::YOUR_CREATURE);

card!(
    index = index::LITTJARA_MIRRORLAKE,
    oracle_id = "3b577179-10d0-43f1-ac17-9dd2d12c965d",
    scryfall_id = "cf1f38de-44b3-42fa-9288-d7c1832ff75f",
    color_identity = ColorSet::from_slice(&[Color::Green, Color::Blue]),
    faces = &[face!(
        name = "Littjara Mirrorlake",
        types = TypeSet::LAND,
        enter_modifiers = &[EnterModifier::Tapped],
    ),],
    coverage = Coverage::Implemented,
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Blue, 1)]),
        activated!(
            cost!("{2}{G}{G}{U}", TapSelf, SacrificeSelf),
            &[Effect::CreateTokenCopyOfTarget {
                mods: &[CopyMod::AddCounter(
                    CounterKind::Plus {
                        power: 1,
                        toughness: 1
                    },
                    1
                )],
                sacrifice_at_next_end_step: false,
            }],
            target = Some(COPY_TARGET),
            timing = ActivationTiming::SorcerySpeed,
        ),
    ],
);
