//! Ba Sing Se — (no cost) — Land
//! Oracle: This land enters tapped unless you control a basic land.
//! Oracle: {T}: Add {G}.
//! Oracle: {2}{G}, {T}: Earthbend 2. Activate only as a sorcery. (Target land you control becomes a 0/0 creature with haste that's still a land. Put two +1/+1 counters on it. When it dies or is exiled, return it to the battlefield tapped.)
//! Set: TLA #266 — Avatar: The Last Airbender | Scryfall ID: bdf3b2be-d0cd-4a3c-a10e-82d32c12d3bd | Oracle ID: de1ae205-ca5b-4d26-8194-ca85f1406e53
// PARTIAL — the entry condition, {G} mana, and earthbend 2's animation and
// counters; the keyword's return clause is NOT SUPPORTED below.

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
    coverage = Coverage::Partial(
        "earthbend's return clause: no effect returns a card from a graveyard or from exile to the battlefield tapped"
    ),
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Green, 1)]),
        // NOT SUPPORTED: "When it dies or is exiled, return it to the
        // battlefield tapped." No `Effect` puts a card from a graveyard or
        // from exile onto the battlefield tapped — `GraveyardToBattlefield`
        // has no tapped form and names a card target — and the animation is
        // an indefinite continuous effect rather than a link, so
        // `Effect::ReturnLinkedToBattlefield` does not reach it either.
        activated!(
            cost!("{2}{G}", TapSelf),
            &[
                Effect::continuous(
                    &Filter::This,
                    Modifier::AddType(TypeSet::CREATURE),
                    Duration::Indefinitely,
                ),
                Effect::continuous(&Filter::This, Modifier::SetPT(0, 0), Duration::Indefinitely,),
                Effect::continuous(
                    &Filter::This,
                    Modifier::AddKeyword(KeywordSet::HASTE),
                    Duration::Indefinitely,
                ),
                Effect::AddCounter {
                    kind: CounterKind::P1P1,
                    amount: Amount::Fixed(2),
                },
            ],
            target = Some(TargetSpec::Object(&Filter::YOUR_LAND)),
            timing = ActivationTiming::SorcerySpeed,
        ),
    ],
);
