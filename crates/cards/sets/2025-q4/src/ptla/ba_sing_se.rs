//! Ba Sing Se — (no cost) — Land
//! Oracle: This land enters tapped unless you control a basic land.
//! Oracle: {T}: Add {G}.
//! Oracle: {2}{G}, {T}: Earthbend 2. Activate only as a sorcery. (Target land you control becomes a 0/0 creature with haste that's still a land. Put two +1/+1 counters on it. When it dies or is exiled, return it to the battlefield tapped.)
//! Set: TLA #266 — Avatar: The Last Airbender | Scryfall ID: bdf3b2be-d0cd-4a3c-a10e-82d32c12d3bd | Oracle ID: de1ae205-ca5b-4d26-8194-ca85f1406e53
// IMPLEMENTED — the entry condition, {G} mana, and earthbend 2 at sorcery
// speed: the animation, the two counters and the return when the land dies
// or is exiled.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::BA_SING_SE,
    oracle_id = "de1ae205-ca5b-4d26-8194-ca85f1406e53",
    scryfall_id = "bdf3b2be-d0cd-4a3c-a10e-82d32c12d3bd",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Ba Sing Se",
        types = TypeSet::LAND,
        enter_modifiers = &[EnterModifier::TappedUnless(&Filter::YOUR_BASIC_LAND)],
    )],
    coverage = Coverage::Implemented,
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Green, 1)]),
        activated!(
            cost!("{2}{G}", TapSelf),
            &[Effect::Earthbend(2)],
            target = Some(TargetSpec::Object(&Filter::YOUR_LAND)),
            timing = ActivationTiming::SorcerySpeed,
        ),
    ],
);
