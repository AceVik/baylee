//! A policy choice, not a game-rule limit: retain old policies and unrelated prices.
use super::{base_view, test_public_object};
use crate::{AIProfile, HeuristicAgent};
use baylee_cards_dsl::{Amount, Effect, PlayerRel};
use baylee_core::{ids::PlayerId, mana::ManaCost};
use baylee_engine::{
    choice::{Pending, PlayerAction, YesNoPrompt},
    engine::DecisionContext,
};
use baylee_view::{CounterEntry, CounterKind, ObjectStatus};

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

#[test]
fn growth_prices_read_the_counter_cost_and_which_sources_can_repeat() {
    let mut view = base_view();
    let mut reservoir = test_public_object(1, "Sol Ring", 0, None);
    reservoir.card = None;
    reservoir.rules = None;
    let token =
        |tier| baylee_cards::tokens::token_id(baylee_cards::tokens::tower::GROWTH_RELICS[3][tier]);
    reservoir.token = Some(token(3));
    view.battlefield.push(reservoir);
    let context = DecisionContext {
        effects: PRICE,
        ..Default::default()
    };
    let agent = |generation| {
        HeuristicAgent::new(AIProfile {
            generation,
            ..AIProfile::TOWER_EXPERT
        })
    };
    let current = agent(AIProfile::OPTIONAL_TOKEN_SATURATION);
    let retained = agent(AIProfile::PAY_LIFE_ACTIVATION_MANA);
    let accepts = |agent: &HeuristicAgent, view: &baylee_view::PlayerView, mana| {
        let pending = Pending::YesNo {
            player: PlayerId::new(0),
            prompt: YesNoPrompt::PayTax { mana },
            source: None,
        };
        agent.act_with_context(view, &pending, &context) == PlayerAction::YesNo(true)
    };
    assert!(
        !accepts(&current, &view, 1),
        "a dry reservoir makes no mana"
    );
    assert!(
        accepts(&retained, &view, 1),
        "retained decisions are frozen"
    );

    view.battlefield[0].counters.push(CounterEntry {
        kind: CounterKind::Charge,
        count: 2,
    });
    assert!(accepts(&current, &view, 2), "two charges buy two mana");
    assert!(!accepts(&current, &view, 3), "do not invent a third charge");
    view.battlefield[0].status = ObjectStatus::TAPPED;
    assert!(
        accepts(&current, &view, 2),
        "a counter-only cost can be paid tapped"
    );

    view.battlefield[0].token = Some(token(1));
    assert!(!accepts(&current, &view, 1), "the Uncommon tier must tap");
    view.battlefield[0].status = ObjectStatus::default();
    view.battlefield[0].counters[0].count = 1;
    assert!(
        !accepts(&current, &view, 1),
        "the Uncommon tier owes two charges"
    );
    view.battlefield[0].counters[0].count = 4;
    assert!(accepts(&current, &view, 1));
    assert!(!accepts(&current, &view, 2), "a tap cost pays at most once");
}
