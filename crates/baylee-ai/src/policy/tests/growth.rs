//! A policy choice, not a game-rule limit: retain old policies and unrelated prices.
use super::{base_view, test_public_object};
use crate::{AIProfile, HeuristicAgent};
use baylee_cards_dsl::{Amount, Effect, PlayerRel};
use baylee_core::{ids::PlayerId, mana::ManaCost};
use baylee_engine::{
    choice::{Pending, PlayerAction, YesNoPrompt},
    engine::DecisionContext,
};

const REWARD: &[Effect] = &[Effect::CreateTokensFor {
    player: PlayerRel::You,
    token: &baylee_cards::tokens::TOWER_SEEDLING,
    amount: Amount::Fixed(1),
}];
const PRICE: &[Effect] = &[Effect::PlayerMayPayThen {
    player: PlayerRel::You,
    mana: Amount::Fixed(1),
    effects: REWARD,
}];
const DRAW: &[Effect] = &[Effect::PlayerMayPayThen {
    player: PlayerRel::You,
    mana: Amount::Fixed(1),
    effects: &[Effect::draw(1)],
}];
const MIXED: &[Effect] = &[Effect::PlayerMayPayThen {
    player: PlayerRel::You,
    mana: Amount::Fixed(1),
    effects: &[REWARD[0], Effect::draw(1)],
}];

#[test]
fn only_new_policy_declines_pure_growth_on_an_ample_own_token_board() {
    let mut view = base_view();
    view.seats[0].mana_pool.colorless = 1;
    for id in 1..=32 {
        let mut creature = test_public_object(id, "Grizzly Bears", 0, None);
        creature.token = Some(baylee_cards::tokens::token_id(
            &baylee_cards::tokens::TOWER_SEEDLING,
        ));
        view.battlefield.push(creature);
    }
    let current = HeuristicAgent::new(AIProfile {
        generation: AIProfile::OPTIONAL_TOKEN_SATURATION,
        ..AIProfile::TOWER_EXPERT
    });
    let original = HeuristicAgent::new(AIProfile {
        generation: AIProfile::PAY_LIFE_ACTIVATION_MANA,
        ..AIProfile::TOWER_EXPERT
    });
    let context = DecisionContext {
        effects: PRICE,
        ..Default::default()
    };
    let pending = Pending::YesNo {
        player: PlayerId::new(0),
        prompt: YesNoPrompt::PayTax { mana: 1 },
        source: None,
    };
    assert_eq!(
        original.act_with_context(&view, &pending, &context),
        PlayerAction::YesNo(true)
    );
    assert_eq!(
        current.act_with_context(&view, &pending, &context),
        PlayerAction::YesNo(false)
    );
    view.battlefield[0].controller = PlayerId::new(1);
    assert_eq!(
        current.act_with_context(&view, &pending, &context),
        PlayerAction::YesNo(true)
    );
    view.battlefield[0].controller = PlayerId::new(0);
    view.battlefield[0].token = None;
    assert_eq!(
        current.act_with_context(&view, &pending, &context),
        PlayerAction::YesNo(true)
    );
    view.battlefield[0].token = Some(baylee_cards::tokens::token_id(
        &baylee_cards::tokens::TOWER_SEEDLING,
    ));
    view.battlefield[0].status = baylee_view::ObjectStatus::PHASED_OUT;
    assert_eq!(
        current.act_with_context(&view, &pending, &context),
        PlayerAction::YesNo(true)
    );
    view.battlefield[0].status = baylee_view::ObjectStatus::MELDED_CARD;
    assert_eq!(
        current.act_with_context(&view, &pending, &context),
        PlayerAction::YesNo(true)
    );
    view.battlefield[0].status = baylee_view::ObjectStatus::TOKEN;
    view.battlefield[0].token = None;
    assert_eq!(
        current.act_with_context(&view, &pending, &context),
        PlayerAction::YesNo(false)
    );
    let draw = DecisionContext {
        effects: DRAW,
        ..Default::default()
    };
    view.seats[0].library_count = 20;
    assert_eq!(
        current.act_with_context(&view, &pending, &draw),
        PlayerAction::YesNo(true)
    );
    let mixed = DecisionContext {
        effects: MIXED,
        ..Default::default()
    };
    assert!(current.pays_optional_price(&view, ManaCost::parse("{1}"), &mixed));
}
