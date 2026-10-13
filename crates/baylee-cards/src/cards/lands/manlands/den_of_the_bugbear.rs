//! Den of the Bugbear — (no cost) — Land
//! Oracle: If you control two or more other lands, this land enters tapped.
//! Oracle: {T}: Add {R}.
//! Oracle: {3}{R}: Until end of turn, this land becomes a 3/2 red Goblin creature with "Whenever this creature attacks, create a 1/1 red Goblin creature token that's tapped and attacking." It's still a land.
//! Set: AFR #254 — Adventures in the Forgotten Realms | Scryfall ID: f231caf8-56c0-4719-a90d-5e5efbee3148 | Oracle ID: f451b8f0-1ff5-4e8d-9f30-9352d83ed687
// IMPLEMENTED — enters tapped with two or more other lands, {T}: Add {R}, and
// {3}{R} animates it into a 3/2 red Goblin creature with the attack trigger
// that creates a 1/1 red Goblin token tapped and attacking.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes::creature;

card!(
    index = index::DEN_OF_THE_BUGBEAR,
    oracle_id = "f451b8f0-1ff5-4e8d-9f30-9352d83ed687",
    scryfall_id = "f231caf8-56c0-4719-a90d-5e5efbee3148",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Den of the Bugbear",
        types = TypeSet::LAND,
        enter_modifiers = &[EnterModifier::TappedUnlessAtMost {
            filter: &Filter::YOUR_LAND,
            at_most: 1,
        }],
    )],
    coverage = Coverage::Implemented,
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Red, 1)]),
        activated!(
            cost!("{3}{R}"),
            &[Effect::Sequence(&[
                Effect::continuous(
                    &Filter::This,
                    Modifier::AddType(TypeSet::CREATURE),
                    Duration::UntilEndOfTurn,
                ),
                Effect::continuous(
                    &Filter::This,
                    Modifier::AddSubtype(creature::GOBLIN),
                    Duration::UntilEndOfTurn,
                ),
                Effect::continuous(
                    &Filter::This,
                    Modifier::SetColor(ColorSet::from_slice(&[Color::Red])),
                    Duration::UntilEndOfTurn,
                ),
                Effect::continuous(
                    &Filter::This,
                    Modifier::SetPT(3, 2),
                    Duration::UntilEndOfTurn,
                ),
                // "… with \"Whenever this creature attacks, create a 1/1 red
                // Goblin creature token that's tapped and attacking.\"" —
                // each activation grants its own instance of the trigger
                // (2021-07-23 ruling).
                Effect::continuous(
                    &Filter::This,
                    Modifier::GrantTriggered {
                        trigger: Trigger::Attacks(&Filter::This),
                        effects: &[Effect::CreateTokenAttacking {
                            token: &crate::generated_tokens::GOBLIN_1_1_RED,
                            amount: Amount::Fixed(1),
                            sacrifice_at_next_end_step: false,
                        }],
                        target: None,
                    },
                    Duration::UntilEndOfTurn,
                ),
            ])],
        ),
    ],
);
