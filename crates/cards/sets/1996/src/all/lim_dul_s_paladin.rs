//! Lim-Dûl's Paladin — {2}{B}{R} — Creature — Human Knight
//! Oracle: Trample
//! Oracle: At the beginning of your upkeep, you may discard a card. If you don't, sacrifice this creature and draw a card.
//! Oracle: Whenever this creature becomes blocked, it gets +6/+3 until end of turn.
//! Oracle: Whenever this creature attacks and isn't blocked, it assigns no combat damage this turn and defending player loses 4 life.
//! Set: ALL #108 — Alliances | Scryfall ID: 44be2d66-359e-4cc1-9670-119cb9c7d5f5 | Oracle ID: c4161335-1150-4f0d-bd2b-9211efb3bea6
use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::LIM_DUL_S_PALADIN,
    oracle_id = "c4161335-1150-4f0d-bd2b-9211efb3bea6",
    scryfall_id = "44be2d66-359e-4cc1-9670-119cb9c7d5f5",
    color_identity = ColorSet::from_slice(&[Color::Black, Color::Red]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Lim-Dûl's Paladin",
        mana_cost = mana!("{2}{B}{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::KNIGHT],
        power = Some(0),
        toughness = Some(3),
        keywords = KeywordSet::TRAMPLE,
    ),],
    abilities = &[
        triggered!(
            Trigger::StepBegin {
                step: StepKind::Upkeep,
                whose: PlayerRel::You,
            },
            &[Effect::PlayerMayPayCostOr {
                player: PlayerRel::You,
                cost: &CostPart::Discard(&Filter::Any),
                effect: &Effect::Sequence(&[Effect::SacrificeSelf, Effect::draw(1)]),
            }],
        ),
        triggered!(
            Trigger::BecomesBlocked,
            &[Effect::ModifySource {
                modifier: Modifier::ModifyPT(6, 3),
                duration: Duration::UntilEndOfTurn,
            }],
        ),
        triggered!(
            Trigger::AttacksAndIsntBlocked(&Filter::This),
            &[
                Effect::ModifySource {
                    modifier: Modifier::AssignsNoCombatDamage,
                    duration: Duration::UntilEndOfTurn,
                },
                Effect::LoseLife {
                    amount: Amount::Fixed(4),
                    target: PlayerRel::DefendingPlayer,
                },
            ],
        ),
    ],
);
