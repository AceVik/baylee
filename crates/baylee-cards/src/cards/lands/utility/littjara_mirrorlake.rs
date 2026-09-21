//! Littjara Mirrorlake — (no cost) — Land
//! Oracle: This land enters tapped.
//! Oracle: {T}: Add {U}.
//! Oracle: {2}{G}{G}{U}, {T}, Sacrifice this land: Create a token that's a copy of target creature you control, except it enters with an additional +1/+1 counter on it. Activate only as a sorcery.
//! Set: NCC #412 — New Capenna Commander | Scryfall ID: cf1f38de-44b3-42fa-9288-d7c1832ff75f | Oracle ID: 3b577179-10d0-43f1-ac17-9dd2d12c965d
// PARTIAL — enters tapped, {T}: Add {U}, and the copy ability at sorcery
// speed; the copy's extra +1/+1 counter is not expressible.

use baylee_cards_dsl::prelude::*;

/// "Target creature you control" — the ability names it twice: once as what
/// it targets, once as what the copy is of.
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
    coverage = Coverage::Partial(
        "the copy token's additional +1/+1 counter: Effect::CreateTokenCopyOf \
         carries no CopyMod list",
    ),
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Blue, 1)]),
        // NOT SUPPORTED: "except it enters with an additional +1/+1 counter
        // on it" — Effect::CreateTokenCopyOf has no mods field, so the copy
        // arrives plain; CopyMod::AddCounter is reachable only through
        // CreateTokenCopyOfEquipped, which copies the creature the source is
        // attached to rather than a target.
        activated!(
            cost!("{2}{G}{G}{U}", TapSelf, SacrificeSelf),
            &[Effect::CreateTokenCopyOf {
                target: Some(COPY_TARGET),
                kicked_bonus: 0,
            }],
            target = Some(COPY_TARGET),
            timing = ActivationTiming::SorcerySpeed,
        ),
    ],
);
