//! A language model deciding for the player whose turn its seat controls
//! (Mindslaver, CR 723.5): a concession for that player is refused in the
//! conversation and the model answers again (CR 723.6); a plan names that
//! player's cards and runs on that player's questions; and a plan of one
//! player's never answers the other's question in the same turn
//! (CR 723.8).

use super::*;
use crate::narrator::tests::controlled::{their_priority, their_turn};
use crate::narrator::tests::{ME, THEM, card};
use baylee_engine::choice::LegalActions;
use baylee_view::PlayerView;

fn ask(view: &PlayerView, pending: Pending, question: u64) -> Request {
    let mut request = request(view.clone(), pending, empty_log());
    request.question = question;
    request
}

fn decides(input: Value) -> Scripted {
    Scripted::ok(claude(&[("toolu_1", "decide", input)]))
}

/// The text blocks of the decision the `n`th call carried.
fn told(provider: &Provider0, n: usize) -> String {
    let seen = provider.seen();
    last_user(&seen[n])
        .iter()
        .filter_map(|block| block["text"].as_str())
        .collect::<Vec<_>>()
        .join("\n")
}

/// The seat's own priority on P2's turn, over its own lands.
fn my_priority_on_their_turn() -> (PlayerView, Pending) {
    let mut view = their_turn();
    view.decision_player = Some(ME);
    view.controlled_hands.clear();
    let pending = Pending::Priority {
        player: ME,
        legal: Box::new(LegalActions {
            can_pass: true,
            mana_abilities: vec![id(21), id(22), id(23)],
            ..LegalActions::default()
        }),
    };
    (view, pending)
}

/// A concession for P2 is no answer: the model is told why in the same
/// conversation and answers again, and no concession is ever sent.
#[tokio::test]
async fn a_concession_for_the_controlled_player_is_refused_and_answered_again() {
    let (base, provider) = stand_in().await;
    provider.script(Scripted::ok(claude(&[(
        "toolu_1",
        "concede",
        json!({"ask": "q12", "reason": "TEST P2 cannot win"}),
    )])));
    provider.script(decides(json!({"ask": "q12", "pick": ["p"]})));
    let mind = mind(&base, Provider::Anthropic, |_| {});
    let answer = mind
        .decide(ask(&their_turn(), their_priority(), 12))
        .await
        .expect("an answer");
    assert_eq!(answer.action, PlayerAction::PassPriority);
    assert_eq!(provider.seen().len(), 2, "asked again once");
    let again = told(&provider, 1);
    assert!(
        again.contains("cannot concede for P2") && again.contains("CR 723.6"),
        "{again}"
    );
    let first = told(&provider, 0);
    assert!(first.contains("You cannot concede for P2."), "{first}");
}

/// The model plays P2's land and plans P2's Unsummon on P2's own Serra
/// Angel: the next question for P2 is answered by the plan, the cast paid
/// with P2's Island, without a call.
#[tokio::test]
async fn a_plan_casts_the_controlled_players_card_with_its_lands() {
    let (base, provider) = stand_in().await;
    provider.script(decides(json!({
        "ask": "q12", "pick": ["a1"], "plan": ["cast #70 -> #45"]
    })));
    let mind = mind(&base, Provider::Anthropic, |_| {});
    let mut view = their_turn();
    let first = mind
        .decide(ask(&view, their_priority(), 12))
        .await
        .expect("an answer");
    assert_eq!(first.action, PlayerAction::PlayLand { card: id(72) });
    assert_eq!(first.planned, 1);

    // P2's Island is in play; the plan's step casts Unsummon.
    view.controlled_hands[0].cards.retain(|c| c.id != id(72));
    let mut island = card(72, "Island");
    island.controller = THEM;
    island.owner = THEM;
    view.battlefield.push(island);
    let pending = Pending::Priority {
        player: THEM,
        legal: Box::new(LegalActions {
            can_pass: true,
            mana_abilities: vec![id(40), id(41), id(42), id(72)],
            payable: vec![id(70), id(71)],
            ..LegalActions::default()
        }),
    };
    let tap = mind
        .decide(ask(&view, pending, 13))
        .await
        .expect("the plan's tap");
    let PlayerAction::ActivateManaAbility { source } = tap.action else {
        panic!("a tap for Unsummon: {:?}", tap.action);
    };
    assert!(
        [id(42), id(72)].contains(&source),
        "one of P2's Islands, not the seat's: {source:?}"
    );
    assert_eq!(provider.seen().len(), 1, "no second call");
}

/// A plan written for P2 stops at the seat's own question in P2's turn,
/// and one written for the seat stops at P2's: each next message says why.
#[tokio::test]
async fn a_plan_never_answers_the_other_players_question() {
    let (base, provider) = stand_in().await;
    provider.script(decides(
        json!({"ask": "q12", "pick": ["a1"], "plan": ["pass"]}),
    ));
    provider.script(decides(
        json!({"ask": "q13", "pick": ["p"], "plan": ["pass"]}),
    ));
    provider.script(decides(json!({"ask": "q14", "pick": ["p"]})));
    let mind = mind(&base, Provider::Anthropic, |_| {});
    mind.decide(ask(&their_turn(), their_priority(), 12))
        .await
        .expect("P2's land");

    let (mine, pending) = my_priority_on_their_turn();
    let own = mind
        .decide(ask(&mine, pending, 13))
        .await
        .expect("the seat's own answer");
    assert_eq!(own.action, PlayerAction::PassPriority);
    assert_eq!(provider.seen().len(), 2, "the model was asked");
    let said = told(&provider, 1);
    assert!(
        said.contains(
            "Plan q12 stopped at step 1 (pass): the plan was for P2's decisions, and this \
             question is your own"
        ),
        "{said}"
    );

    let theirs = mind
        .decide(ask(&their_turn(), their_priority(), 14))
        .await
        .expect("P2's answer");
    assert_eq!(theirs.action, PlayerAction::PassPriority);
    assert_eq!(provider.seen().len(), 3, "the model was asked");
    let said = told(&provider, 2);
    assert!(
        said.contains(
            "Plan q13 stopped at step 1 (pass): the plan was for your own decisions, and this \
             question is one you decide for P2"
        ),
        "{said}"
    );
}
