//! Restless Ridgeline — (no cost) — Land
//! Oracle: This land enters tapped.
//! Oracle: {T}: Add {R} or {G}.
//! Oracle: {2}{R}{G}: This land becomes a 3/4 red and green Dinosaur creature until end of turn. It's still a land.
//! Oracle: Whenever this land attacks, another target attacking creature gets +2/+0 until end of turn. Untap that creature.
//! Set: LCI #283 — The Lost Caverns of Ixalan | Scryfall ID: abde5bed-dc4e-4b2b-820c-18d4d0cf8042 | Oracle ID: 4c0f4a63-586a-4dde-9621-b0dd9118b2e5
// IMPLEMENTED — enters tapped; {T} for {R} or {G}; the animation (a Dinosaur
// creature that is still a land, red and green, 3/4 until end of turn); the
// attack trigger pumping another attacking creature +2/+0 and untapping it.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::RESTLESS_RIDGELINE,
    oracle_id = "4c0f4a63-586a-4dde-9621-b0dd9118b2e5",
    scryfall_id = "abde5bed-dc4e-4b2b-820c-18d4d0cf8042",
    color_identity = ColorSet::from_slice(&[Color::Green, Color::Red]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Restless Ridgeline",
        types = TypeSet::LAND,
        enter_modifiers = &[EnterModifier::Tapped],
    ),],
    abilities = &[
        mana_ability!(&[Effect::mana_choice(&[ManaColor::Red, ManaColor::Green])]),
        activated!(
            cost!("{2}{R}{G}"),
            &[
                Effect::continuous(
                    &Filter::This,
                    Modifier::AddType(TypeSet::CREATURE),
                    Duration::UntilEndOfTurn,
                ),
                Effect::continuous(
                    &Filter::This,
                    Modifier::AddSubtype(subtypes::creature::DINOSAUR),
                    Duration::UntilEndOfTurn,
                ),
                Effect::continuous(
                    &Filter::This,
                    Modifier::AddColor(ColorSet::from_slice(&[Color::Red, Color::Green])),
                    Duration::UntilEndOfTurn,
                ),
                Effect::continuous(
                    &Filter::This,
                    Modifier::SetPT(3, 4),
                    Duration::UntilEndOfTurn,
                ),
            ],
        ),
        triggered!(
            Trigger::Attacks(&Filter::This),
            &[
                Effect::PumpTarget {
                    power: Amount::Fixed(2),
                    toughness: Amount::Fixed(0),
                    keywords: KeywordSet::EMPTY,
                    duration: Duration::UntilEndOfTurn,
                },
                Effect::UntapTarget,
            ],
            targets = Some(TargetReq::one(TargetSpec::Object(&f!(
                another attacking CREATURE
            )))),
        ),
    ],
);
