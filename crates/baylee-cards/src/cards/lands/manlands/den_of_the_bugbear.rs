//! Den of the Bugbear — (no cost) — Land
//! Oracle: If you control two or more other lands, this land enters tapped.
//! Oracle: {T}: Add {R}.
//! Oracle: {3}{R}: Until end of turn, this land becomes a 3/2 red Goblin creature with "Whenever this creature attacks, create a 1/1 red Goblin creature token that's tapped and attacking." It's still a land.
//! Set: AFR #254 — Adventures in the Forgotten Realms | Scryfall ID: f231caf8-56c0-4719-a90d-5e5efbee3148 | Oracle ID: f451b8f0-1ff5-4e8d-9f30-9352d83ed687
// PARTIAL — the enters-tapped count, the {T} mana ability, and the {3}{R}
// animation (still a land, plus the creature type, the Goblin subtype, red
// and 3/2); the ability the animation grants is left out — see the
// NOT SUPPORTED line beside the activated ability.

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
    coverage = Coverage::Partial(
        "the animated land's granted ability — \"Whenever this creature attacks, create a 1/1 red Goblin creature token that's tapped and attacking\" — cannot be said: no `Effect` creates a token tapped and attacking, and a Goblin created any other way would play wrongly",
    ),
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Red, 1)]),
        // NOT SUPPORTED: "… with \"Whenever this creature attacks, create a
        // 1/1 red Goblin creature token that's tapped and attacking.\"" — the
        // granted trigger would need a token that is tapped and attacking,
        // which no `Effect` produces. The 1/1 red Goblin exists
        // (`GOBLIN_1_1_RED`), but made untapped and outside combat it would
        // be a blocker the card never prints.
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
                    Modifier::AddColor(ColorSet::from_slice(&[Color::Red])),
                    Duration::UntilEndOfTurn,
                ),
                Effect::continuous(
                    &Filter::This,
                    Modifier::SetPT(3, 2),
                    Duration::UntilEndOfTurn,
                ),
            ])],
        ),
    ],
);
