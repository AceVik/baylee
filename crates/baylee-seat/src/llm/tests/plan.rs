//! A plan the model writes once, run by the seat against each question
//! after it, without a call (`docs/llm-protocol.md` §"Plans"): the
//! fetchland turn of the recorded games in one call, and every reason a
//! plan stops, each proving that nothing of the plan was sent and that the
//! next message says why.

use super::*;
use crate::narrator::tests::{ME, card, golden, in_hand};
use baylee_core::mana::ManaColor;
use baylee_engine::choice::{ChoicePrompt, LegalActions, YesNoPrompt};
use baylee_view::{PlayerView, StackItem, TargetingContext};
use std::fmt::Write as _;

/// A priority offering `lands`, the spells `castable` from the pool and
/// `abilities`, over the board's sources and whatever `mana` adds.
fn offer(lands: &[u32], castable: &[u32], abilities: &[(u32, u32)], mana: &[u32]) -> Pending {
    Pending::Priority {
        player: ME,
        legal: Box::new(LegalActions {
            unpaid_abilities: vec![],
            payable: vec![],
            spell_increases: vec![],
            activation_increases: vec![],
            can_pass: true,
            lands: lands.iter().map(|&s| id(s)).collect(),
            castable: castable.iter().map(|&s| id(s)).collect(),
            mana_abilities: [21, 22, 23, 30]
                .iter()
                .chain(mana)
                .map(|&s| id(s))
                .collect(),
            abilities: abilities.iter().map(|&(s, i)| (id(s), i)).collect(),
            suspendable: Vec::new(),
            granted_actions: vec![],
        }),
    }
}

/// The board with Birds of Paradise for the Mountain, so that the Bolt's
/// tap asks a colour, and Lightning Bolt and Verdant Catacombs in hand.
fn fetch_board() -> PlayerView {
    let (mut view, _) = board();
    view.battlefield.retain(|o| o.id != id(23));
    view.battlefield.push(card(23, "Birds of Paradise"));
    view.hand = vec![
        in_hand(50, "Lightning Bolt"),
        in_hand(209, "Verdant Catacombs"),
    ];
    view
}

fn ask(view: &PlayerView, pending: Pending, question: u64) -> Request {
    let mut request = request(view.clone(), pending, empty_log());
    request.question = question;
    request
}

/// The model's answer: a call of `decide` with `input`.
fn decides(input: Value) -> Scripted {
    Scripted::ok(claude(&[("toolu_1", "decide", input)]))
}

/// The text of the decision the `n`th call carried.
fn told(provider: &Provider0, n: usize) -> String {
    let seen = provider.seen();
    last_user(&seen[n])
        .iter()
        .filter_map(|block| block["text"].as_str())
        .collect::<Vec<_>>()
        .join("\n")
}

/// The fetchland turn of 01a10d40 (`docs/llm-protocol.md` §"Plans"): play
/// the land, crack it, fetch a Swamp from two, cast Lightning Bolt with
/// the tap that asks a colour, name its target ahead, pass. Four calls
/// before plans; one now. The next call is told what ran.
#[tokio::test]
#[allow(clippy::too_many_lines)] // nine questions, in the order the table asks them
async fn a_fetchland_turn_is_one_call() {
    let (base, provider) = stand_in().await;
    provider.script(decides(json!({
        "ask": "q12", "pick": ["a1"],
        "plan": ["activate #209", "choose Swamp", "cast #50 -> P2", "pass"]
    })));
    provider.script(decides(json!({"ask": "q21", "pick": ["p"]})));
    let mind = mind(&base, Provider::Anthropic, |_| {});
    let mut view = fetch_board();
    let decide = |request: Request| {
        let mind = &mind;
        async move { mind.decide(request).await.expect("an answer").action }
    };

    // q12: the model plays the land and writes the plan.
    let first = decide(ask(&view, offer(&[209], &[], &[(31, 0)], &[]), 12)).await;
    assert_eq!(first, PlayerAction::PlayLand { card: id(209) });

    // q13: the land is in play; the plan cracks it.
    view.hand.retain(|c| c.id != id(209));
    view.battlefield.push(card(209, "Verdant Catacombs"));
    let crack = decide(ask(&view, offer(&[], &[], &[(209, 0), (31, 0)], &[]), 13)).await;
    assert_eq!(
        crack,
        PlayerAction::ActivateAbility {
            source: id(209),
            ability_index: 0
        }
    );

    // q14: the land's own ability is on the stack, and the next step
    // answers what it asks: the seat passes for it.
    view.battlefield.retain(|o| o.id != id(209));
    view.seats[0].life = 16;
    let mut ability = card(210, "Verdant Catacombs");
    ability.stack_item = Some(StackItem::Ability {
        source: id(209),
        ability: None,
        text: None,
        rules: None,
        token: None,
    });
    view.stack = vec![ability];
    let waited = decide(ask(&view, offer(&[], &[], &[(31, 0)], &[]), 14)).await;
    assert_eq!(waited, PlayerAction::PassPriority);

    // q15: the search shows two Swamps, one card, and a Forest.
    view.looking_at = vec![card(312, "Swamp"), card(313, "Swamp"), card(314, "Forest")];
    let search = Pending::ChooseCards {
        player: ME,
        options: vec![id(312), id(313), id(314)],
        min: 0,
        max: 1,
        prompt: ChoicePrompt::SearchLibrary,
        total: None,
    };
    let chose = decide(ask(&view, search, 15)).await;
    assert_eq!(
        chose,
        PlayerAction::ChooseObjects {
            objects: vec![id(312)]
        }
    );

    // q16: the Swamp is in; the Bolt is cast by tapping the Birds for red.
    view.looking_at.clear();
    view.stack.clear();
    view.battlefield.push(card(312, "Swamp"));
    let tap = decide(ask(&view, offer(&[], &[], &[(23, 0), (31, 0)], &[312]), 16)).await;
    assert!(
        matches!(
            tap,
            PlayerAction::ActivateManaAbility { source }
                | PlayerAction::ActivateAbility { source, .. } if source == id(23)
        ),
        "{tap:?}"
    );

    // q17: the Birds ask which colour: the tap's.
    let colour = Pending::ChooseColor {
        player: ME,
        options: vec![
            ManaColor::White,
            ManaColor::Blue,
            ManaColor::Black,
            ManaColor::Red,
            ManaColor::Green,
        ],
    };
    let red = decide(ask(&view, colour, 17)).await;
    assert_eq!(red, PlayerAction::ChooseColor(ManaColor::Red));

    // q18: the red is floating; the cast goes out.
    view.seats[0].mana_pool.red = 1;
    let cast = decide(ask(
        &view,
        offer(&[], &[50], &[(23, 0), (31, 0)], &[312]),
        18,
    ))
    .await;
    assert_eq!(cast, PlayerAction::CastSpell { card: id(50) });

    // q19: the target named ahead.
    view.seats[0].mana_pool.red = 0;
    view.hand.clear();
    let mut bolt = card(50, "Lightning Bolt");
    bolt.stack_item = Some(StackItem::Spell);
    view.stack = vec![bolt.clone()];
    view.targeting = Some(TargetingContext {
        source: bolt,
        text: None,
        whole_spell: true,
        second: false,
        batch_count: 1,
    });
    let targets = Pending::ChooseTargets {
        player: ME,
        options: vec![id(30), id(31), id(45), id(46)],
        player_options: vec![ME, THEM],
        min: 1,
        max: 1,
        reason: TargetPrompt::Targets,
    };
    let aimed = decide(ask(&view, targets, 19)).await;
    assert_eq!(
        aimed,
        PlayerAction::ChooseTargets {
            objects: Vec::new(),
            players: vec![THEM]
        }
    );

    // q20: the Bolt on the stack; the last step passes.
    view.targeting = None;
    let passed = decide(ask(&view, offer(&[], &[], &[(23, 0), (31, 0)], &[312]), 20)).await;
    assert_eq!(passed, PlayerAction::PassPriority);
    assert_eq!(provider.seen().len(), 1, "eight questions, one call");

    // q21: the Bolt resolved; the model is asked, and told what ran.
    view.stack.clear();
    let next = decide(ask(&view, offer(&[], &[], &[(23, 0), (31, 0)], &[312]), 21)).await;
    assert_eq!(next, PlayerAction::PassPriority);
    assert_eq!(provider.seen().len(), 2);
    let text = told(&provider, 1);
    assert!(
        text.contains(
            "Plan q12 ran: activate #209 · passed 1 window · choose Swamp · cast #50 -> P2 · pass."
        ),
        "{text}"
    );
}

/// One way a plan stops: what the seat is asked after the model wrote
/// `["activate #209", "choose Swamp"]` at q12.
struct Fault {
    name: &'static str,
    view: PlayerView,
    pending: Pending,
    /// What the model answers the question then.
    answer: &'static str,
}

/// Every reason a plan stops (§"Plans"): the model is called at once, the
/// plan sends nothing, and the message names the step and the reason in
/// the report's words (`golden/plan_report.txt`). A stale id is refused,
/// never mapped to whatever holds its slot now.
#[tokio::test]
async fn a_plan_stops_and_asks_at_anything_it_does_not_answer_exactly() {
    let mut view = fetch_board();
    view.hand.retain(|c| c.id != id(209));
    view.battlefield.push(card(209, "Verdant Catacombs"));
    let cracking = offer(&[], &[], &[(209, 0), (31, 0)], &[]);
    let with = |change: &dyn Fn(&mut PlayerView)| {
        let mut view = view.clone();
        change(&mut view);
        view
    };
    let faults = [
        Fault {
            name: "another kind of question",
            view: view.clone(),
            pending: Pending::YesNo {
                player: ME,
                prompt: YesNoPrompt::PayLifeOrEnterTapped { amount: 2 },
                source: None,
            },
            answer: "no",
        },
        Fault {
            name: "a stale id",
            view: with(&|v| v.battlefield.retain(|o| o.id != id(209))),
            pending: offer(&[], &[], &[(31, 0)], &[]),
            answer: "p",
        },
        Fault {
            name: "an opponent's spell",
            view: with(&|v| {
                let mut counter = card(61, "Counterspell");
                counter.controller = THEM;
                counter.stack_item = Some(StackItem::Spell);
                v.stack = vec![counter];
            }),
            pending: cracking.clone(),
            answer: "p",
        },
        Fault {
            name: "a payment owed",
            view: with(&|v| {
                v.owed = Some(baylee_core::mana::ManaPayment::Fixed(
                    "{1}".parse().expect("a cost"),
                ));
                v.awaiting = Some(ME);
            }),
            pending: cracking.clone(),
            answer: "p",
        },
        Fault {
            name: "the turn's end",
            view: with(&|v| v.turn = 8),
            pending: cracking.clone(),
            answer: "p",
        },
    ];
    let mut report = String::new();
    for fault in faults {
        let (base, provider) = stand_in().await;
        provider.script(decides(json!({
            "ask": "q12", "pick": ["a1"], "plan": ["activate #209", "choose Swamp"]
        })));
        let mind = mind(&base, Provider::Anthropic, |_| {});
        let first = mind
            .decide(ask(&fetch_board(), offer(&[209], &[], &[(31, 0)], &[]), 12))
            .await
            .expect("the first answer");
        assert_eq!(first.action, PlayerAction::PlayLand { card: id(209) });
        provider.script(decides(json!({"ask": "q13", "pick": [fault.answer]})));
        let request = ask(&fault.view, fault.pending, 13);
        let answer = mind.decide(request).await.expect("the model's answer");
        assert_eq!(
            provider.seen().len(),
            2,
            "{}: the model is asked",
            fault.name
        );
        assert!(
            !answer
                .note
                .as_deref()
                .unwrap_or_default()
                .contains("\"plan\""),
            "{}: the plan sent nothing",
            fault.name
        );
        let text = told(&provider, 1);
        let line = text
            .lines()
            .find(|l| l.starts_with("Plan q12"))
            .unwrap_or_else(|| panic!("{}: no report in\n{text}", fault.name));
        assert!(line.contains("at step 1 (activate #209)"), "{line}");
        let _ = writeln!(report, "{}: {line}", fault.name);
    }
    golden("plan_report.txt", &report);
}

/// A step the table refuses ends the plan: the next message says which
/// and why, and the model answers the question again.
#[tokio::test]
async fn a_refused_step_ends_the_plan_and_says_so() {
    let (base, provider) = stand_in().await;
    provider.script(decides(json!({
        "ask": "q12", "pick": ["a1"], "plan": ["activate #209", "choose Swamp"]
    })));
    provider.script(decides(json!({"ask": "q13", "pick": ["p"]})));
    let mind = mind(&base, Provider::Anthropic, |_| {});
    let mut view = fetch_board();
    mind.decide(ask(&view, offer(&[209], &[], &[(31, 0)], &[]), 12))
        .await
        .expect("the first answer");
    view.hand.retain(|c| c.id != id(209));
    view.battlefield.push(card(209, "Verdant Catacombs"));
    let cracking = offer(&[], &[], &[(209, 0), (31, 0)], &[]);
    let crack = mind
        .decide(ask(&view, cracking.clone(), 13))
        .await
        .expect("the plan's step");
    assert_eq!(provider.seen().len(), 1);
    let mut again = ask(&view, cracking, 13);
    again.retry = Some(Refusal {
        answer: crack.action,
        reason: "TEST-refused".into(),
        by: RefusedBy::Table,
    });
    let answer = mind.decide(again).await.expect("the model's answer");
    assert_eq!(answer.action, PlayerAction::PassPriority);
    let text = told(&provider, 1);
    assert!(
        text.contains("Plan q12 stopped at step 1 (activate #209): the table refused it")
            && text.contains("TEST-refused"),
        "{text}"
    );
}
