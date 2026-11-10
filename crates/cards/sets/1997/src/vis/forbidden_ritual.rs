//! Forbidden Ritual — {2}{B}{B} — Sorcery
//! Oracle: Sacrifice a nontoken permanent. If you do, target opponent loses 2 life unless that player sacrifices a permanent of their choice or discards a card. You may repeat this process any number of times.
//! Set: VIS #60 — Visions | Scryfall ID: f5327e6d-db4e-4b44-a00e-b764e80b8946 | Oracle ID: 4b5d9c76-8be8-4e25-b978-9750df3d8c15
// IMPLEMENTED — sacrifice a nontoken permanent; if you do, the opponent chooses between
// sacrificing a permanent (their choice), discarding a card, and losing 2 life; then you may
// go again, as often as you like. The opponent's choice is asked as two questions in a row
// (sacrifice? then discard?), which reaches the same three outcomes.

use baylee_cards_dsl::prelude::*;

/// "a nontoken permanent" the caster controls.
const YOUR_NONTOKEN_PERMANENT: Filter =
    Filter::And(&[Filter::ControlledByYou, Filter::Not(&Filter::IsToken)]);

/// One pass of the process: "Sacrifice a nontoken permanent. If you do, target opponent loses
/// 2 life unless that player sacrifices a permanent of their choice or discards a card."
const THE_PROCESS: Effect = Effect::SacrificeOneThen {
    filter: &YOUR_NONTOKEN_PERMANENT,
    if_it_was: &Filter::Any,
    then: &[Effect::PlayerMayPayCostOr {
        player: PlayerRel::Chosen,
        cost: &CostPart::Sacrifice(&Filter::Any),
        effect: &Effect::PlayerMayPayCostOr {
            player: PlayerRel::Chosen,
            cost: &CostPart::Discard(&Filter::Any),
            effect: &Effect::LoseLife {
                amount: Amount::Fixed(2),
                target: PlayerRel::Chosen,
            },
        },
    }],
};

card!(
    index = index::FORBIDDEN_RITUAL,
    oracle_id = "4b5d9c76-8be8-4e25-b978-9750df3d8c15",
    scryfall_id = "f5327e6d-db4e-4b44-a00e-b764e80b8946",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Forbidden Ritual",
        mana_cost = mana!("{2}{B}{B}"),
        types = TypeSet::SORCERY,
    ),],
    abilities = &[spell!(
        &[
            // The first pass is not optional.
            THE_PROCESS,
            // "You may repeat this process any number of times": asked after each pass, and
            // gone round again only when a nontoken permanent of yours was sacrificed in it.
            Effect::RepeatWhile {
                effects: &[Effect::MayDo {
                    effects: &[THE_PROCESS],
                }],
                condition: Condition::AmountAtLeast(
                    Amount::ThisWay(ThisWay::Died(&YOUR_NONTOKEN_PERMANENT)),
                    1,
                ),
            },
        ],
        targets = Some(TargetReq::one(TargetSpec::AnyOpponent)),
    )],
);
