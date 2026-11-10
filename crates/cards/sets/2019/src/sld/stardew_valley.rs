//! Stardew Valley — (no cost) — Land
//! Oracle: {T}: Add {C}.
//! Oracle: {2}, {T}, Tap an untapped creature you control: Create a Food token.
//! Oracle: {3}, {T}: Choose target permanent you control. Draw a card, then another player of your choice may gain control of that permanent. Activate only as a sorcery.
//! Set: SLD #2801 — Secret Lair Drop | Scryfall ID: 9979db80-83f6-41ed-aea2-0c222e923add | Oracle ID: 6a4ee425-b3b8-487d-866c-9e2d73682466
// IMPLEMENTED — mana, Food with both tap costs, and the sorcery-speed draw
// followed by a chosen other player’s optional permanent control transfer.

use crate::tokens;
use baylee_cards_dsl::prelude::*;

card!(
    index = index::STARDEW_VALLEY,
    oracle_id = "6a4ee425-b3b8-487d-866c-9e2d73682466",
    scryfall_id = "9979db80-83f6-41ed-aea2-0c222e923add",
    faces = &[face!(name = "Stardew Valley", types = TypeSet::LAND,),],
    coverage = Coverage::Implemented,
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        activated!(
            cost!("{2}", TapSelf, TapOther(&Filter::YOUR_CREATURE)),
            &[Effect::CreateToken {
                token: &tokens::FOOD
            }]
        ),
        activated!(
            cost!("{3}", TapSelf),
            &[
                Effect::draw(1),
                Effect::ChoosePlayerFrom {
                    among: PlayerRel::EachOtherPlayer
                },
                Effect::MayDoFor {
                    player: PlayerRel::Chosen,
                    effects: &[Effect::GainControlAll {
                        filter: &Filter::Reference,
                        to: PlayerRel::Chosen,
                    }],
                },
            ],
            target = Some(TargetSpec::Object(&Filter::ControlledByYou)),
            timing = ActivationTiming::SorcerySpeed,
        ),
    ],
);
