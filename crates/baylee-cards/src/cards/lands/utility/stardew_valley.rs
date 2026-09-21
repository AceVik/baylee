//! Stardew Valley — (no cost) — Land
//! Oracle: {T}: Add {C}.
//! Oracle: {2}, {T}, Tap an untapped creature you control: Create a Food token.
//! Oracle: {3}, {T}: Choose target permanent you control. Draw a card, then another player of your choice may gain control of that permanent. Activate only as a sorcery.
//! Set: SLD #2801 — Secret Lair Drop | Scryfall ID: 9979db80-83f6-41ed-aea2-0c222e923add | Oracle ID: 6a4ee425-b3b8-487d-866c-9e2d73682466
// IMPLEMENTED — {C}, the {2} ability with its tapped creature, and the {3}
// ability's target and draw; the hand-over has no variant (see NOT SUPPORTED).

use crate::tokens;
use baylee_cards_dsl::prelude::*;

card!(
    index = index::STARDEW_VALLEY,
    oracle_id = "6a4ee425-b3b8-487d-866c-9e2d73682466",
    scryfall_id = "9979db80-83f6-41ed-aea2-0c222e923add",
    faces = &[face!(name = "Stardew Valley", types = TypeSet::LAND,),],
    coverage = Coverage::Partial(
        "the {3} ability cannot hand the targeted permanent to a chosen other \
         player: no variant asks anyone but the controller the \"may\", and \
         the chosen seat has no field to be picked through",
    ),
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        activated!(
            cost!("{2}", TapSelf, TapOther(&Filter::YOUR_CREATURE)),
            &[Effect::CreateToken {
                token: &tokens::FOOD
            }]
        ),
        // NOT SUPPORTED: "then another player of your choice may gain control
        // of that permanent" — Effect::ChangeController names a seat but
        // offers no door for the chosen player to be chosen through (the
        // ability's one target is the permanent), and MayDo asks the
        // controller and nobody else.
        activated!(
            cost!("{3}", TapSelf),
            &[Effect::draw(1)],
            target = Some(TargetSpec::Object(&Filter::ControlledByYou)),
            timing = ActivationTiming::SorcerySpeed,
        ),
    ],
);
