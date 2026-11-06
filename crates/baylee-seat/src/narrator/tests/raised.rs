//! A price above the printed one, offered whenever the lands pay it
//! (`wake/raised.rs`), on tables a real engine dealt: what the menu says, and
//! that answering it reaches the engine's own question with that way on it.

use super::*;
use crate::wake::tests::played::position;
use baylee_engine::choice::{CastModeKind, YesNoPrompt};
use baylee_gamehost::Session;

fn asked(session: &Session) -> Request {
    let (pending, view) = session.view_for(ME).expect("seat 0 is asked");
    request(view, pending, LogTail::default())
}

/// The table, and the question it asks seat 0.
fn table(mine: &[&str], hand: &[&str], theirs: &[&str]) -> (Session, Request) {
    let (session, _, _) = position(mine, hand, theirs);
    let request = asked(&session);
    (session, request)
}

fn lines(request: &Request) -> Vec<String> {
    let wake = Narrator::new(&request.context).wake(request, &[], None);
    wake.text.lines().map(str::to_string).collect()
}

fn offers(request: &Request, words: &str) -> Vec<String> {
    lines(request)
        .into_iter()
        .filter(|line| line.contains(words) && line.contains("taps "))
        .collect()
}

/// Answers the option on `line`, sends its taps and its cast.
fn press(session: &mut Session, request: &Request, line: &str) {
    let wake = Narrator::new(&request.context).wake(request, &[], None);
    let id = line.split_whitespace().next().expect("an option id");
    let answer = wake
        .menu
        .resolve(&Decision {
            ask: Some("q12".into()),
            pick: vec![id.to_string()],
            ..Default::default()
        })
        .expect("the option is on the menu");
    let Act::Taps { steps, then } = answer.act else {
        panic!("taps first, got {:?}", answer.act);
    };
    for step in &steps {
        session
            .act_slice(ME, tap(step), 0)
            .expect("each tap is offered");
    }
    session.act_slice(ME, then, 0).expect("the cast is offered");
}

const FORESTS: [&str; 14] = ["Forest"; 14];

/// The report: Journey of Discovery with fourteen lands up. The plain price
/// is offered as it was, and beside it the entwined one, in full, and tapping
/// for it brings the engine's mode question with entwine on it.
#[test]
fn entwine_is_offered_at_its_whole_price_and_reaches_the_mode_question() {
    let (mut session, request) = table(&FORESTS, &["Journey of Discovery"], &[]);
    let plain = offers(&request, "{2}{G}");
    assert_eq!(plain.len(), 1, "{plain:?}");
    let entwined = offers(&request, "with entwine for {4}{G}{G}");
    assert_eq!(entwined.len(), 1, "{:#?}", lines(&request));
    press(&mut session, &request, &entwined[0]);

    let question = asked(&session);
    let Pending::ChooseCastMode { options, .. } = &question.pending else {
        panic!("the mode question, got {:?}", question.pending);
    };
    let both = options
        .iter()
        .find(|option| matches!(option.kind, CastModeKind::Modes(0b11)))
        .expect("entwine is on the question");
    assert_eq!(both.cost, baylee_core::mana!("{4}{G}{G}"));
    let words = lines(&question).join("\n");
    assert!(
        words.contains("Cast it with entwine for {4}{G}{G}"),
        "{words}"
    );
    assert!(!words.contains("Choose 3 modes"), "{words}");
}

/// The hint is for cards that print a price of their own: most cards, at
/// most priorities, cost the offer nothing.
#[test]
fn a_card_with_one_price_is_not_in_the_engines_list() {
    let (_, _, legal) = position(&FORESTS[..3], &["Rampant Growth"], &[]);
    assert_eq!(legal.payable.len(), 1);
    assert!(legal.cast_ways.is_empty(), "{:?}", legal.cast_ways);
}

/// Five lands pay `{2}{G}` and not `{4}{G}{G}`: no play is invented.
#[test]
fn too_little_mana_offers_only_the_plain_price() {
    let (_, request) = table(&FORESTS[..5], &["Journey of Discovery"], &[]);
    assert_eq!(offers(&request, "Journey of Discovery").len(), 1);
    assert!(offers(&request, "entwine").is_empty());
    let (_, request) = table(&FORESTS[..2], &["Journey of Discovery"], &[]);
    assert!(offers(&request, "Journey of Discovery").is_empty());
}

/// Rite of Replication's kicker is asked after the cast begins, so it is
/// offered as a price and the yes-or-no follows with the mana already up.
#[test]
fn a_kicker_is_offered_at_its_whole_price_and_the_question_follows() {
    const ISLANDS: [&str; 9] = ["Island"; 9];
    let creature = ["Grizzly Bears"];
    let (mut session, request) = table(&ISLANDS, &["Rite of Replication"], &creature);
    let kicked = offers(&request, "with its kicker for {7}{U}{U}");
    assert_eq!(kicked.len(), 1, "{:#?}", lines(&request));
    assert_eq!(
        offers(&request, "{2}{U}{U} (taps").len(),
        1,
        "the plain one stays"
    );
    press(&mut session, &request, &kicked[0]);
    for _ in 0..3 {
        let question = asked(&session);
        match &question.pending {
            Pending::YesNo {
                prompt: YesNoPrompt::Kicker,
                ..
            } => return,
            Pending::ChooseTargets { options, .. } => {
                let targets = vec![options[0]];
                session
                    .act_slice(
                        ME,
                        PlayerAction::ChooseTargets {
                            objects: targets,
                            players: Vec::new(),
                        },
                        0,
                    )
                    .expect("the Bears are a target");
            }
            other => panic!("the kicker's question, got {other:?}"),
        }
    }
    panic!("the kicker was not asked");
}

/// Capsize's {3} is a price of its own, its whole price named, the plain one
/// stays beside it, and pressing it reaches the engine's question with the
/// mana floating. The question then says what buyback does.
#[test]
fn a_buyback_is_offered_at_its_whole_price_and_the_question_follows() {
    const ISLANDS: [&str; 6] = ["Island"; 6];
    let (mut session, request) = table(&ISLANDS, &["Capsize"], &["Grizzly Bears"]);
    let bought = offers(&request, "with its buyback for {4}{U}{U}");
    assert_eq!(bought.len(), 1, "{:#?}", lines(&request));
    assert_eq!(
        offers(&request, "{1}{U}{U} (taps").len(),
        1,
        "the plain one stays"
    );
    press(&mut session, &request, &bought[0]);
    let question = asked(&session);
    assert!(
        matches!(
            question.pending,
            Pending::YesNo {
                prompt: YesNoPrompt::Buyback { .. },
                ..
            }
        ),
        "{:?}",
        question.pending
    );
    let text = lines(&question).join("\n");
    assert!(text.contains("Pay its buyback ({3})?"), "{text}");
    assert!(
        text.contains("goes back to your hand as it resolves"),
        "{text}"
    );
    assert!(text.contains("countered"), "{text}");
    assert!(
        text.contains("The spell: Capsize: \"Buyback {3}"),
        "the printed sentence is shown too: {text}"
    );
}

#[test]
fn too_little_mana_offers_no_buyback() {
    const ISLANDS: [&str; 5] = ["Island"; 5];
    let (_, request) = table(&ISLANDS, &["Capsize"], &["Grizzly Bears"]);
    assert!(offers(&request, "buyback").is_empty());
    assert_eq!(offers(&request, "{1}{U}{U} (taps").len(), 1);
}

/// A buyback with no mana in it costs the plain cast nothing more, so it has
/// no price of its own: the question comes to the plain cast, and names it.
#[test]
fn a_buyback_of_parts_is_asked_of_the_plain_cast() {
    const SWAMPS: [&str; 4] = ["Swamp"; 4];
    let (mut session, request) = table(&SWAMPS, &["Slaughter"], &["Grizzly Bears"]);
    assert!(offers(&request, "buyback").is_empty());
    let plain = offers(&request, "{2}{B}{B} (taps");
    assert_eq!(plain.len(), 1, "{:#?}", lines(&request));
    press(&mut session, &request, &plain[0]);
    let text = lines(&asked(&session)).join("\n");
    assert!(text.contains("Pay its buyback (pay 4 life)?"), "{text}");
}

#[test]
fn too_little_mana_offers_no_kicker() {
    const ISLANDS: [&str; 8] = ["Island"; 8];
    let (_, request) = table(&ISLANDS, &["Rite of Replication"], &["Grizzly Bears"]);
    assert_eq!(offers(&request, "Rite of Replication").len(), 1);
    assert!(offers(&request, "kicker").is_empty());
}

/// Overload is a mode with a price of its own, and only the engine knows
/// which modes are legal: with nothing to bounce, `{1}{U}` is no way to cast
/// Cyclonic Rift, and the lands are not tapped for it.
#[test]
fn overload_is_offered_and_a_mode_with_no_target_is_not() {
    const ISLANDS: [&str; 7] = ["Island"; 7];
    let bears = ["Grizzly Bears"];
    let (_, request) = table(&ISLANDS, &["Cyclonic Rift"], &bears);
    assert_eq!(
        offers(&request, "for {6}{U}").len(),
        1,
        "{:#?}",
        lines(&request)
    );
    assert_eq!(
        offers(&request, "Cyclonic Rift").len(),
        2,
        "{:#?}",
        lines(&request)
    );
    let (_, request) = table(&ISLANDS, &["Cyclonic Rift"], &[]);
    assert_eq!(
        offers(&request, "Cyclonic Rift").len(),
        1,
        "{:#?}",
        lines(&request)
    );
    assert_eq!(offers(&request, "for {6}{U}").len(), 1);
    let (_, request) = table(&ISLANDS[..2], &["Cyclonic Rift"], &[]);
    assert!(offers(&request, "Cyclonic Rift").is_empty());
}

/// A kicker with targets of its own stands on the mode question.
#[test]
fn a_kicker_with_targets_of_its_own_is_offered_from_the_engines_list() {
    let lands = ["Forest", "Forest", "Swamp", "Swamp"];
    let (mut session, request) = table(&lands, &["Tear Asunder"], &["Sol Ring"]);
    let kicked = offers(&request, "kicked for {2}{B}{G}");
    assert_eq!(kicked.len(), 1, "{:#?}", lines(&request));
    press(&mut session, &request, &kicked[0]);
    let question = asked(&session);
    assert!(
        matches!(&question.pending, Pending::ChooseTargets { .. })
            || matches!(&question.pending, Pending::ChooseCastMode { options, .. }
                if options.iter().any(|o| o.kind == CastModeKind::Kicked)),
        "{:?}",
        question.pending
    );
}

/// Replicate is bounded by the pool, so it is asked only when the mana is up
/// first: once, and as often as the lands pay.
#[test]
fn replicate_is_offered_once_and_as_often_as_the_lands_pay() {
    let lands: Vec<PublicObject> = (70..75).map(|slot| card(slot, "Island")).collect();
    let ids: Vec<ObjectId> = lands.iter().map(|land| land.id).collect();
    let mut view = ViewBuilder::new(2).with_battlefield(0, lands).build();
    view.hand = vec![in_hand(90, "Lose Focus")];
    view.stack = vec![card(95, "Grizzly Bears")];
    let request = request(
        view,
        Pending::Priority {
            player: ME,
            legal: Box::new(LegalActions {
                can_pass: true,
                mana_abilities: ids,
                payable: vec![id(90)],
                ..Default::default()
            }),
        },
        LogTail::default(),
    );
    assert_eq!(offers(&request, "replicated once for {1}{U}{U}").len(), 1);
    assert_eq!(
        offers(&request, "replicated up to 3 times for {1}{U}{U}{U}{U}").len(),
        1,
        "{:#?}",
        lines(&request)
    );
    assert_eq!(
        offers(&request, "Lose Focus").len(),
        3,
        "and the plain price"
    );
}

/// A plan names a card, not a price: the printed one is what it casts, and
/// a card on two options is no longer ambiguous.
#[test]
fn a_plan_step_takes_the_printed_price() {
    let (_, request) = table(&FORESTS, &["Journey of Discovery"], &[]);
    let card = request.view.hand[0].id;
    let wake = Narrator::new(&request.context).wake(&request, &[], None);
    let step = menu::Step::parse(&format!("cast {}", tag(card))).expect("a step");
    let decision = wake
        .menu
        .plan_decision(&step, &request)
        .expect("the card is offered");
    let answer = wake
        .menu
        .resolve(&decision)
        .expect("it is one of the options");
    let Act::Taps { steps, .. } = answer.act else {
        panic!("taps first");
    };
    assert_eq!(steps.len(), 3, "the printed price, not the entwined one");
}

/// Rith's Charm with no nonbasic land to destroy: its first mode is not on
/// offer, and a plan's `mode m2` still means the second printed mode, the
/// Saprolings, as the list says. It once took the third (prevent damage),
/// because the number counted the modes offered.
#[test]
fn a_plan_names_a_mode_by_its_printed_number() {
    let (mut session, request) = table(
        &["Mountain", "Forest", "Plains"],
        &["Rith's Charm"],
        &["Forest"],
    );
    let lines_now = offers(&request, "Rith's Charm");
    assert_eq!(lines_now.len(), 1, "{:#?}", lines(&request));
    press(&mut session, &request, &lines_now[0]);

    let question = asked(&session);
    let Pending::ChooseCastMode { options, .. } = &question.pending else {
        panic!("the mode question, got {:?}", question.pending);
    };
    assert_eq!(options.len(), 2, "no nonbasic land to destroy: {options:?}");
    let wake = Narrator::new(&question.context).wake(&question, &[], None);
    let text = wake.text.clone();
    assert!(!text.contains("  m1 "), "{text}");
    assert!(text.contains("m2") && text.contains("m3"), "{text}");

    let answer = |written: &str| {
        let step = menu::Step::parse(written).expect("a step");
        wake.menu
            .plan_decision(&step, &question)
            .and_then(|decision| wake.menu.resolve(&decision).map_err(menu::Stop::NotOffered))
    };
    let second = answer("mode m2").expect("the Saprolings are offered");
    let Act::Now(PlayerAction::ChooseMode(at)) = second.act else {
        panic!("a mode answer, got {:?}", second.act);
    };
    assert!(
        matches!(options[at].kind, CastModeKind::Mode(1)),
        "m2 is the printed second mode: {:?}",
        options[at]
    );
    assert!(
        answer("mode m1").is_err(),
        "the first mode is not offered, so the plan stops"
    );
}

/// Four Plains and a second Mouth of Ronom pay `{4}{S}` for the first
/// (CR 107.4h: the other Mouth's mana is snow mana). A model reported the
/// sacrifice unoffered: the planner refused every cost with `{S}`.
#[test]
fn a_mouth_of_ronom_is_offered_with_another_mouth_to_pay_the_snow() {
    let (_, request) = table(
        &[
            "Plains",
            "Plains",
            "Plains",
            "Plains",
            "Mouth of Ronom",
            "Mouth of Ronom",
        ],
        &[],
        &["Forest", "Grizzly Bears"],
    );
    let found = offers(&request, "Mouth of Ronom");
    assert!(
        found.iter().any(|line| line.contains("4 damage")),
        "{:#?}",
        lines(&request)
    );
}
