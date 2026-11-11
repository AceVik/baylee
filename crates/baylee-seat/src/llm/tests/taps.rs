//! An option's taps through a cost beyond the tap: Lazotep Quarry's "{T},
//! Sacrifice a creature: Add one mana of any color" asks what to sacrifice
//! as it is tapped (CR 605.3a, 601.2h), and the seat dropped the option's
//! taps there, with the cast never made and the mana floating away (Carbon
//! c3, hc-expert-c game-0002 q129-q136). Dealt by a real engine; the model
//! answers through a stand-in provider.

use super::plan::{decides, told};
use super::*;
use crate::narrator::tests::ME;
use crate::wake::tests::played::position;
use baylee_core::ids::ObjectId;
use baylee_engine::choice::{ChoicePrompt, timeout_answer};
use baylee_gamehost::Session;

/// Seat 0's question as the bridge hands it to the mind, numbered `question`.
fn asked(session: &Session, question: u64) -> Request {
    let (pending, view) = session.view_for(ME).expect("seat 0 is asked");
    let mut request = request(view, pending, empty_log());
    request.question = question;
    request
}

/// The id of the option whose words start with `label`.
fn option(request: &Request, label: &str) -> String {
    let menu = crate::narrator::Narrator::new(&request.context).menu(request);
    menu.offered()
        .iter()
        .find(|offered| offered.text.starts_with(label))
        .unwrap_or_else(|| panic!("no option {label:?} in {:?}", menu.offered()))
        .id
        .clone()
}

/// Answers seat 0's questions with `mind` and every other with its timeout
/// answer, from question `from` on, until `card` is on the stack; how many
/// questions seat 0 was asked.
async fn play_until_cast(mind: &ApiMind, session: &mut Session, card: ObjectId, from: u64) -> u64 {
    let mut question = from;
    for _ in 0..40 {
        if session.observe_view(ME).stack.iter().any(|o| o.id == card) {
            return question - from;
        }
        let pending = session.pending().clone();
        let player = pending.asked().expect("the table stays playable");
        let action = if player == ME {
            let request = asked(session, question);
            question += 1;
            mind.decide(request).await.expect("an answer").action
        } else {
            timeout_answer(&pending).expect("an answer for the other seat")
        };
        session
            .act_slice(player, action, 0)
            .expect("the table takes what the seat sends");
    }
    panic!("the cast never reached the stack");
}

/// The board of the report, smaller: the Dragon's `{R}` is only Lazotep
/// Quarry's, which sacrifices a creature as it is tapped.
fn quarry_board(creatures: &[&'static str]) -> (Session, ObjectId) {
    let mut mine = vec!["Plains", "Forest", "Forest", "Island", "Lazotep Quarry"];
    mine.extend_from_slice(creatures);
    let (session, view, _) = position(&mine, &["Adult Gold Dragon"], &[]);
    let dragon = view
        .hand
        .iter()
        .find(|c| c.name == "Adult Gold Dragon")
        .expect("the Dragon in hand")
        .id;
    (session, dragon)
}

/// One creature to sacrifice: the cost's question has one answer, and the
/// option's taps and its cast go on without a call. The option says what
/// the Quarry's tap costs.
#[tokio::test]
async fn an_option_s_taps_go_through_a_cost_with_one_answer() {
    let (mut session, dragon) = quarry_board(&["Zebra Unicorn"]);
    let first = asked(&session, 1);
    let pick = option(&first, "Cast Adult Gold Dragon");
    let label = crate::narrator::Narrator::new(&first.context)
        .menu(&first)
        .offered()
        .iter()
        .find(|o| o.id == pick)
        .map(|o| o.text.clone())
        .expect("the option");
    assert!(
        label.contains("Sacrifice a creature"),
        "the label names the sacrifice: {label}"
    );
    let (base, provider) = stand_in().await;
    provider.script(decides(json!({"ask": "q1", "pick": [pick]})));
    let mind = mind(&base, Provider::Anthropic, |_| {});
    play_until_cast(&mind, &mut session, dragon, 1).await;
    assert_eq!(provider.seen().len(), 1, "one call for the whole cast");
    let view = session.observe_view(ME);
    assert!(
        !view
            .battlefield
            .iter()
            .any(|o| o.name.as_ref() == "Zebra Unicorn"),
        "the Unicorn was the cost"
    );
}

/// Two creatures: which one goes is the model's, so the question is its,
/// told that the taps are under way; after its answer the colour, the
/// rest of the taps and the cast follow without a call.
#[tokio::test]
async fn a_cost_with_a_choice_is_asked_and_the_taps_go_on_after_it() {
    let (mut session, dragon) = quarry_board(&["Zebra Unicorn", "Grizzly Bears"]);
    let first = asked(&session, 1);
    let pick = option(&first, "Cast Adult Gold Dragon");
    let (base, provider) = stand_in().await;
    provider.script(decides(json!({"ask": "q1", "pick": [pick]})));
    let mind = mind(&base, Provider::Anthropic, |_| {});
    let mut question = 1;
    // Up to the sacrifice: the Quarry is tapped and its cost asks.
    for _ in 0..12 {
        if let Pending::ChooseCards {
            prompt: ChoicePrompt::CostSacrifice,
            ..
        } = session.pending()
        {
            break;
        }
        let request = asked(&session, question);
        question += 1;
        let action = mind.decide(request).await.expect("an answer").action;
        session.act_slice(ME, action, 0).expect("the step is taken");
    }
    let Pending::ChooseCards {
        prompt: ChoicePrompt::CostSacrifice,
        options,
        ..
    } = session.pending().clone()
    else {
        panic!("the Quarry asks its sacrifice, not {:?}", session.pending());
    };
    let view = session.observe_view(ME);
    let bears = view
        .battlefield
        .iter()
        .find(|o| o.name.as_ref() == "Grizzly Bears")
        .expect("the Bears")
        .id;
    assert!(options.contains(&bears));
    provider.script(decides(json!({
        "ask": format!("q{question}"),
        "pick": [format!("#{}", bears.slot())]
    })));
    let asked_then = play_until_cast(&mind, &mut session, dragon, question).await;
    assert!(
        asked_then >= 3,
        "the colour, the taps and the cast came after"
    );
    assert_eq!(provider.seen().len(), 2, "the sacrifice was the model's");
    let text = told(&provider, 1);
    assert!(
        text.contains("are under way") && text.contains("Cast Adult Gold Dragon"),
        "the model is told the cast goes on: {text}"
    );
    let view = session.observe_view(ME);
    assert!(
        view.battlefield
            .iter()
            .any(|o| o.name.as_ref() == "Zebra Unicorn")
    );
    assert!(!view.battlefield.iter().any(|o| o.id == bears));
}
