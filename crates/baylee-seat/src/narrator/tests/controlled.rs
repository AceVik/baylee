//! A decision the seat makes for the player whose turn it controls
//! (Mindslaver, CR 723.5), told as one: the line under the header, P2's
//! hand under P2's name, options that cast with P2's cards and tap P2's
//! lands, P2's own permanents and P2's own face among what may be chosen,
//! "your" only for the seat's own things, and no concession (CR 723.6).

use super::*;
use crate::narrator::menu::Step as PlanStep;
use baylee_core::ids::Defender;
use baylee_view::SharedHand;

/// P2's turn 8, P2's precombat main phase, the seat (P1) deciding for P2:
/// P2's Plains #40 and #41 and Island #42 untapped, Serra Angel #45 and a
/// face-down #46; P2's hand of Unsummon #70, Pacifism #71 and Island #72,
/// seen through control; the seat's own hand and board as in [`board`].
pub(crate) fn their_turn() -> PlayerView {
    let (mut view, _) = board();
    view.turn = 8;
    view.active = THEM;
    view.awaiting = Some(ME);
    view.decision_player = Some(THEM);
    if let Some(plains) = view.battlefield.iter_mut().find(|o| o.id == id(41)) {
        plains.status = ObjectStatus::NONE;
    }
    view.controlled_hands = vec![SharedHand {
        player: THEM,
        public: false,
        cards: vec![
            in_hand(70, "Unsummon"),
            in_hand(71, "Pacifism"),
            in_hand(72, "Island"),
        ],
    }];
    view
}

/// P2's priority: P2's land to play, P2's lands' mana, P2's two spells
/// castable once paid.
pub(crate) fn their_priority() -> Pending {
    Pending::Priority {
        player: THEM,
        legal: Box::new(LegalActions {
            can_pass: true,
            lands: vec![id(72)],
            mana_abilities: vec![id(40), id(41), id(42)],
            payable: vec![id(70), id(71)],
            ..LegalActions::default()
        }),
    }
}

fn no_log() -> LogTail {
    LogTail {
        from: 0,
        entries: Vec::new(),
    }
}

fn pick(question: u64, ids: &[&str]) -> Decision {
    Decision {
        ask: Some(format!("q{question}")),
        pick: ids.iter().map(|id| (*id).to_string()).collect(),
        ..Decision::default()
    }
}

/// The steps an answer's taps make, by the object each taps.
fn tapped(act: &Act) -> (Vec<ObjectId>, PlayerAction) {
    match act {
        Act::Taps { steps, then } => (steps.iter().map(|s| s.source).collect(), then.clone()),
        Act::Now(action) => (Vec::new(), action.clone()),
    }
}

#[test]
fn a_controlled_priority_reads_as_its_golden_text() {
    let request = request(their_turn(), their_priority(), no_log());
    let wake = Narrator::new(&request.context).wake(&request, &[], None);
    golden("controlled_priority.txt", &wake.text);
    assert_eq!(
        wake.headline,
        "q12 · turn 8, P2's turn · precombat main phase · Priority for P2"
    );
    for seat_s_own in ["Cast Lightning Bolt", "Cast Giant Growth", "Mountain #23)"] {
        assert!(!wake.text.contains(seat_s_own), "{seat_s_own}");
    }
}

/// The model's pick of P2's land and P2's spells resolves to P2's
/// actions, the spells paid with P2's lands.
#[test]
fn the_pick_of_a_controlled_players_card_resolves_to_its_action() {
    let request = request(their_turn(), their_priority(), no_log());
    let menu = Narrator::new(&request.context).menu(&request);
    let land = menu.resolve(&pick(12, &["a1"])).expect("the land");
    assert_eq!(land.act, Act::Now(PlayerAction::PlayLand { card: id(72) }));
    let unsummon = menu.resolve(&pick(12, &["a2"])).expect("Unsummon");
    assert_eq!(
        tapped(&unsummon.act),
        (vec![id(42)], PlayerAction::CastSpell { card: id(70) }),
        "P2's Island pays for P2's Unsummon"
    );
    let pacifism = menu.resolve(&pick(12, &["a3"])).expect("Pacifism");
    let (taps, then) = tapped(&pacifism.act);
    assert_eq!(then, PlayerAction::CastSpell { card: id(71) });
    assert_eq!(taps.len(), 2);
    assert!(
        taps.iter().all(|t| [id(40), id(41), id(42)].contains(t)),
        "only P2's lands: {taps:?}"
    );
}

/// A plan step that names P2's card is read against P2's offer, and its
/// targets ahead may name P2's own creature.
#[test]
fn a_plan_step_names_the_controlled_players_card() {
    let request = request(their_turn(), their_priority(), no_log());
    let menu = Narrator::new(&request.context).menu(&request);
    let step = PlanStep::parse("cast #71 -> #45").expect("a step");
    let decision = menu.plan_decision(&step, &request).expect("P2's Pacifism");
    assert_eq!(decision.pick, vec!["a3".to_string()]);
    let resolved = menu.resolve(&decision).expect("it resolves");
    assert_eq!(
        tapped(&resolved.act).1,
        PlayerAction::CastSpell { card: id(71) }
    );
    assert_eq!(resolved.hint.map(|h| h.objects), Some(vec![id(45)]));
    let mine = PlanStep::parse("cast #50").expect("a step");
    assert!(
        menu.plan_decision(&mine, &request).is_err(),
        "the seat's own Bolt is no option for P2"
    );
}

/// P2's attack: P2's creatures, the seat itself the defender.
#[test]
fn a_controlled_attack_reads_as_its_golden_text() {
    let mut view = their_turn();
    view.phase = baylee_view::Phase::Combat;
    view.step = baylee_view::Step::DeclareAttackers;
    let pending = Pending::ChooseAttackers {
        player: THEM,
        attackers: vec![id(45), id(46)],
        defenders: vec![Defender::Player(ME)],
        required: Vec::new(),
        alone: Vec::new(),
        minimum_required: None,
        limits: Vec::new(),
    };
    let request = request(view, pending, no_log());
    let wake = Narrator::new(&request.context).wake(&request, &[], None);
    golden("controlled_attackers.txt", &wake.text);
    let attack = wake
        .menu
        .resolve(&Decision {
            attacks: vec![("#45".into(), "P1".into())],
            ..pick(12, &[])
        })
        .expect("P2's Angel attacks the seat");
    assert_eq!(
        attack.act,
        Act::Now(PlayerAction::DeclareAttackers {
            attackers: vec![(id(45), Defender::Player(ME))]
        })
    );
}

/// P2's Unsummon asks for its target: P2's own creatures are among the
/// choices, and the model may name one.
#[test]
fn a_controlled_target_choice_reads_as_its_golden_text() {
    let mut view = their_turn();
    let mut unsummon = card(70, "Unsummon");
    unsummon.controller = THEM;
    unsummon.owner = THEM;
    unsummon.stack_item = Some(baylee_view::StackItem::Spell);
    view.controlled_hands[0].cards.retain(|c| c.id != id(70));
    view.stack = vec![unsummon.clone()];
    view.targeting = Some(baylee_view::TargetingContext {
        source: unsummon,
        text: None,
        whole_spell: true,
        second: false,
        batch_count: 1,
    });
    let pending = Pending::ChooseTargets {
        player: THEM,
        options: vec![id(30), id(31), id(45), id(46)],
        player_options: Vec::new(),
        min: 1,
        max: 1,
        reason: baylee_engine::choice::TargetPrompt::Targets,
    };
    let request = request(view, pending, no_log());
    let wake = Narrator::new(&request.context).wake(&request, &[], None);
    golden("controlled_targets.txt", &wake.text);
    let own = wake
        .menu
        .resolve(&pick(12, &["#45"]))
        .expect("P2's own Angel");
    assert_eq!(
        own.act,
        Act::Now(PlayerAction::ChooseTargets {
            objects: vec![id(45)],
            players: Vec::new()
        })
    );
}

/// The player decided for may be chosen like any other, its own face
/// included, and is labelled as the one decided for.
#[test]
fn the_controlled_player_is_among_the_players_to_choose() {
    let pending = Pending::ChoosePlayer {
        player: THEM,
        options: vec![ME, THEM],
    };
    let request = request(their_turn(), pending, no_log());
    let wake = Narrator::new(&request.context).wake(&request, &[], None);
    assert!(
        wake.text
            .contains("P2  opponent (you decide for this player), 14 life"),
        "{}",
        wake.text
    );
    assert!(wake.text.contains("P1  you, 17 life"), "{}", wake.text);
    let chosen = wake.menu.resolve(&pick(12, &["P2"])).expect("P2 itself");
    assert_eq!(chosen.act, Act::Now(PlayerAction::ChoosePlayer(THEM)));
}

/// A concession is no answer in a decision for P2 (CR 723.6): the menu
/// says why. In the seat's own decision it is its own to make.
#[test]
fn a_concession_is_no_answer_for_a_controlled_player() {
    let concede = Decision {
        concede: Some("TEST hopeless".into()),
        ..pick(12, &[])
    };
    let request = request(their_turn(), their_priority(), no_log());
    let menu = Narrator::new(&request.context).menu(&request);
    let why = menu.resolve(&concede).expect_err("refused");
    assert!(
        why.contains("cannot concede for P2") && why.contains("CR 723.6"),
        "{why}"
    );
    let own = a_priority_of_the_seats_own();
    let menu = Narrator::new(&own.context).menu(&own);
    assert_eq!(
        menu.resolve(&concede).expect("its own").act,
        Act::Now(PlayerAction::Concede)
    );
}

/// P2's cleanup discard chooses from P2's hand, named as P2's.
#[test]
fn a_controlled_discard_chooses_from_the_controlled_hand() {
    let mut view = their_turn();
    view.phase = baylee_view::Phase::Ending;
    view.step = baylee_view::Step::Cleanup;
    let pending = Pending::DiscardChoice {
        player: THEM,
        count: 1,
    };
    let request = request(view, pending, no_log());
    let wake = Narrator::new(&request.context).wake(&request, &[], None);
    let question = wake.text.split("QUESTION").nth(1).expect("a question");
    assert!(
        question.contains("P2's hand is over its maximum size"),
        "{question}"
    );
    assert!(question.contains("Choose from P2's hand:"), "{question}");
    assert!(question.contains("#71 Pacifism"), "{question}");
    assert!(!question.contains("Lightning Bolt"), "{question}");
    let discard = wake.menu.resolve(&pick(12, &["#72"])).expect("P2's Island");
    assert_eq!(
        discard.act,
        Act::Now(PlayerAction::ChooseObjects {
            objects: vec![id(72)]
        })
    );
}

fn a_priority_of_the_seats_own() -> Request {
    let (view, log) = board();
    let pending = priority(&view);
    request(view, pending, log)
}
