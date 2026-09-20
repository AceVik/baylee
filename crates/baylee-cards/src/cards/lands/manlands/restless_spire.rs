//! Restless Spire — (no cost) — Land
//! Oracle: This land enters tapped.
//! Oracle: {T}: Add {U} or {R}.
//! Oracle: {U}{R}: Until end of turn, this land becomes a 2/1 blue and red Elemental creature with "During your turn, this creature has first strike." It's still a land.
//! Oracle: Whenever this land attacks, scry 1.
//! Set: FRC #82 — Reality Fracture Commander | Scryfall ID: 30ecfd44-8dd4-4ae3-9076-14799f8a9a14 | Oracle ID: 0ca4e80e-c19c-4b74-b531-c5a4dc5a8ba9
// PARTIAL — enters tapped, {T} for {U} or {R}, the {U}{R} manland animation
// and the attack trigger; the animation's granted "During your turn, this
// creature has first strike" has no DSL variant and is left off.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes::creature;

card!(
    index = index::RESTLESS_SPIRE,
    oracle_id = "0ca4e80e-c19c-4b74-b531-c5a4dc5a8ba9",
    scryfall_id = "30ecfd44-8dd4-4ae3-9076-14799f8a9a14",
    color_identity = ColorSet::from_slice(&[Color::Red, Color::Blue]),
    coverage = Coverage::Partial(
        "the manland animation's granted clause \"During your turn, this creature \
         has first strike\" — the DSL has no turn-conditional keyword grant, \
         neither on a created effect nor as a static ability"
    ),
    faces = &[face!(
        name = "Restless Spire",
        types = TypeSet::LAND,
        enter_modifiers = &[EnterModifier::Tapped],
    )],
    abilities = &[
        mana_ability!(&[Effect::mana_choice(&[ManaColor::Blue, ManaColor::Red])]),
        activated!(
            cost!("{U}{R}"),
            &[
                // NOT SUPPORTED: "During your turn, this creature has first strike."
                // No turn-conditional keyword grant exists, so the animated land is a
                // 2/1 blue and red Elemental with no first strike.
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
                    Modifier::AddColor(ColorSet::from_slice(&[Color::Blue, Color::Red])),
                    Duration::UntilEndOfTurn,
                ),
                Effect::continuous(
                    &Filter::This,
                    Modifier::SetPT(2, 1),
                    Duration::UntilEndOfTurn,
                ),
            ]
        ),
        triggered!(Trigger::Attacks(&Filter::This), &[Effect::scry(1)]),
    ],
);
