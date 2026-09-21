//! Lavaclaw Reaches — (no cost) — Land
//! Oracle: This land enters tapped.
//! Oracle: {T}: Add {B} or {R}.
//! Oracle: {1}{B}{R}: Until end of turn, this land becomes a 2/2 black and red Elemental creature with "{X}: This creature gets +X/+0 until end of turn." It's still a land.
//! Set: UMA #245 — Ultimate Masters | Scryfall ID: 409fcd0c-8449-4623-8bf0-cd7ca0937a4a | Oracle ID: 340de307-982a-4b26-9dc0-99113dc766cd
// PARTIAL — enters tapped, taps for {B} or {R}, and animates into a 2/2
// black and red Elemental that is still a land (AddType adds, so the land
// type is never taken away). The animated creature's granted
// "{X}: This creature gets +X/+0" ability is not built: granting an
// activated ability to an object at run time has no engine support.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes::creature;

card!(
    index = index::LAVACLAW_REACHES,
    oracle_id = "340de307-982a-4b26-9dc0-99113dc766cd",
    scryfall_id = "409fcd0c-8449-4623-8bf0-cd7ca0937a4a",
    color_identity = ColorSet::from_slice(&[Color::Black, Color::Red]),
    faces = &[face!(
        name = "Lavaclaw Reaches",
        types = TypeSet::LAND,
        enter_modifiers = &[EnterModifier::Tapped],
    )],
    coverage = Coverage::Partial(
        "the animated land's granted \"{X}: This creature gets +X/+0 until end of turn\" ability is dropped — the engine cannot grant an activated ability"
    ),
    abilities = &[
        mana_ability!(&[Effect::mana_choice(&[ManaColor::Black, ManaColor::Red])]),
        // NOT SUPPORTED: the animated creature's "{X}: This creature gets
        // +X/+0 until end of turn." Granting an activated ability to an
        // object at run time is not implemented (docs/card-dsl.md,
        // "Explicitly not supported yet"), so the ability comes off the card
        // rather than being written and silently skipped.
        activated!(
            cost!("{1}{B}{R}"),
            &[
                Effect::continuous(
                    &Filter::This,
                    Modifier::AddType(TypeSet::CREATURE),
                    Duration::UntilEndOfTurn,
                ),
                Effect::continuous(
                    &Filter::This,
                    Modifier::AddSubtype(creature::ELEMENTAL),
                    Duration::UntilEndOfTurn,
                ),
                Effect::continuous(
                    &Filter::This,
                    Modifier::AddColor(ColorSet::from_slice(&[Color::Black, Color::Red])),
                    Duration::UntilEndOfTurn,
                ),
                Effect::continuous(
                    &Filter::This,
                    Modifier::SetPT(2, 2),
                    Duration::UntilEndOfTurn,
                ),
            ],
        ),
    ],
);
