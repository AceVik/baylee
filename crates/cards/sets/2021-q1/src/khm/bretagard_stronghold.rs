//! Bretagard Stronghold — (no cost) — Land
//! Oracle: This land enters tapped.
//! Oracle: {T}: Add {G}.
//! Oracle: {G}{W}{W}, {T}, Sacrifice this land: Put a +1/+1 counter on each of up to two target creatures you control. They gain vigilance and lifelink until end of turn. Activate only as a sorcery.
//! Set: MOC #392 — March of the Machine Commander | Scryfall ID: 67733bb9-9151-4ddc-b104-e48328bd1b28 | Oracle ID: c792229b-4a0f-48d5-93e5-60bd4cae9c42

use baylee_cards_dsl::prelude::*;

card!(
    index = index::BRETAGARD_STRONGHOLD,
    oracle_id = "c792229b-4a0f-48d5-93e5-60bd4cae9c42",
    scryfall_id = "67733bb9-9151-4ddc-b104-e48328bd1b28",
    color_identity = ColorSet::from_slice(&[Color::Green, Color::White]),
    faces = &[face!(
        name = "Bretagard Stronghold",
        types = TypeSet::LAND,
        enter_modifiers = &[EnterModifier::Tapped],
    )],
    coverage = Coverage::Implemented,
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Green, 1)]),
        // Both effects walk every chosen target. With none chosen the ability
        // still resolves and does nothing: a counter never lands on the land,
        // because the ability has a target requirement even when it is
        // answered with nothing.
        activated!(
            cost!("{G}{W}{W}", TapSelf, SacrificeSelf),
            &[
                Effect::AddCounter {
                    kind: CounterKind::P1P1,
                    amount: Amount::Fixed(1),
                },
                Effect::PumpTarget {
                    power: Amount::Fixed(0),
                    toughness: Amount::Fixed(0),
                    keywords: KeywordSet::VIGILANCE.union(KeywordSet::LIFELINK),
                    duration: Duration::UntilEndOfTurn,
                },
            ],
            targets = Some(TargetReq::up_to(
                TargetSpec::Object(&Filter::YOUR_CREATURE),
                2
            )),
            timing = ActivationTiming::SorcerySpeed,
        ),
    ],
);
