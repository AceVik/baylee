//! Teferi, Time Raveler — {1}{W}{U} — Legendary Planeswalker — Teferi
//! Oracle: Each opponent can cast spells only any time they could cast a sorcery.
//! Oracle: +1: Until your next turn, you may cast sorcery spells as though they had flash.
//! Oracle: −3: Return up to one target artifact, creature, or enchantment to its owner's hand. Draw a card.
//! Set: RVR #232 — Ravnica Remastered | Scryfall ID: 662fe50f-d75c-422c-8c6c-1f9b5c4ba21f | Oracle ID: ae7604bb-4818-45a3-960c-cf3d83f15964
// PARTIAL — timing lock + −3 implemented; +1 needs UntilYourNextTurn (M2+).

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes::planeswalker;

static BOUNCE_TARGET: Filter =
    Filter::Or(&[Filter::ARTIFACT, Filter::CREATURE, Filter::ENCHANTMENT]);

card! {
    index: 166,
    oracle_id: "ae7604bb-4818-45a3-960c-cf3d83f15964",
    scryfall_id: "662fe50f-d75c-422c-8c6c-1f9b5c4ba21f",
    faces: &[face! {
        name: "Teferi, Time Raveler",
        mana_cost: baylee_core::mana!("{1}{W}{U}"),
        types: TypeSet::PLANESWALKER,
        supertypes: SupertypeSet::LEGENDARY,
        subtypes: &[planeswalker::TEFERI],
        loyalty: Some(4),
    }],
    color_identity: ColorSet::from_slice(&[Color::White, Color::Blue]),
    commander: CommanderRule::Legendary,
    coverage: Coverage::Implemented,
    abilities: &[
        AbilityDef::Static(StaticAbility {
            layer: Layer::Text,
            filter: Filter::Any,
            modifier: Modifier::OpponentsCastAsSorcery,
            cross_zone: false,
        }),
        loyalty!(1, &[Effect::CreateContinuousEffect {
                layer: Layer::Text,
                filter: &Filter::Any,
                modifier: Modifier::SorceriesHaveFlash,
                duration: Duration::UntilYourNextTurn,
            }]),
        loyalty!(-3, &[
                Effect::ReturnToHand {
                    target: TargetSpec::Object(&BOUNCE_TARGET),
                },
                Effect::DrawCards {
                    amount: Amount::Fixed(1),
                },
            ], target: Some(TargetSpec::Object(&BOUNCE_TARGET))),
    ],
}
