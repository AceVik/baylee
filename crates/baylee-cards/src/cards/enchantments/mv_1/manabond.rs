//! Manabond — {G} — Enchantment
//! Oracle: At the beginning of your end step, you may reveal your hand and put all land cards from it onto the battlefield. If you do, discard your hand.
//! Set: TPR #179 — Tempest Remastered | Scryfall ID: 6f94ab94-5a8f-4422-8927-0b7da85c3119 | Oracle ID: 482006a0-bb2a-4533-891b-2195da5e0f01

use baylee_cards_dsl::prelude::*;

card!(
    index = index::MANABOND,
    oracle_id = "482006a0-bb2a-4533-891b-2195da5e0f01",
    scryfall_id = "6f94ab94-5a8f-4422-8927-0b7da85c3119",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Partial(
        "Putting all land cards from your hand onto the battlefield simultaneously is not supported; at the beginning of your end step, you may put up to one land card from your hand onto the battlefield and discard the rest of your hand."
    ),
    faces = &[face!(
        name = "Manabond",
        mana_cost = mana!("{G}"),
        types = TypeSet::ENCHANTMENT,
    ),],
    abilities = &[triggered!(
        Trigger::StepBegin {
            step: StepKind::End,
            whose: PlayerRel::You,
        },
        &[Effect::MayDo {
            effects: &[
                Effect::PutFromHandOntoBattlefield {
                    tapped: false,
                    filter: &Filter::LAND,
                    mana_value: None,
                    optional: true,
                },
                Effect::DiscardHand {
                    who: PlayerRel::You,
                },
            ],
        }],
    )],
);
