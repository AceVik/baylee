#[allow(clippy::wildcard_imports)] // shared integration fixture vocabulary
use super::*;

#[test]
fn the_ai_casts_both_commanders_with_independent_cast_counts() {
    // Freeform setup exercises the multi-commander engine path without
    // claiming these two cards form a legal Partner pair. Real pairing
    // validation fixtures are tracked in docs/ai-coverage-todo.md.
    let mut preset = position(
        &[],
        &["Forest", "Plains", "Island", "Swamp", "Plains", "Island"],
    );
    preset.seats[0].commanders = vec![
        entry("Katara, the Fearless"),
        entry("Aminatou, the Fateshifter"),
    ];
    let mut engine = Engine::new(&preset, RegistryLookup).unwrap();
    let agent = HeuristicAgent::new(AIProfile::EXPERT);
    for seq in 0..150 {
        let pending = engine.pending();
        let seat = pending_player(pending).expect("fixture remains live");
        let view = asked_view(engine.state(), seat, seq, pending);
        if view
            .battlefield
            .iter()
            .filter(|o| o.commander && o.controller == PlayerId::new(0))
            .count()
            == 2
        {
            assert_eq!(
                engine.state().commanders[0]
                    .iter()
                    .map(|c| c.casts)
                    .collect::<Vec<_>>(),
                vec![1, 1]
            );
            return;
        }
        let action = match pending {
            Pending::Mulligan { .. } => PlayerAction::MulliganKeep,
            Pending::Priority { .. } if seat == PlayerId::new(1) => PlayerAction::PassPriority,
            _ => agent.act_with_context(&view, pending, &engine.decision_context()),
        };
        engine
            .apply(seat, action)
            .expect("each commander's payment and choice are legal");
    }
    panic!("six correctly coloured lands must deploy both three-mana commanders");
}

#[test]
fn an_x_draw_spell_pays_for_x_and_draws_for_its_caster() {
    let preset = position(
        &["Commander's Insight"],
        &["Island", "Island", "Island", "Island", "Island"],
    );
    let mut engine = Engine::new(&preset, RegistryLookup).unwrap();
    let agent = HeuristicAgent::new(AIProfile::EXPERT);
    let mut chosen_x = false;
    for seq in 0..100 {
        let pending = engine.pending();
        let seat = pending_player(pending).unwrap();
        let view = asked_view(engine.state(), seat, seq, pending);
        if chosen_x && view.hand.len() == 2 && seat == PlayerId::new(0) {
            assert_eq!(view.seats[1].hand_count, 0);
            return;
        }
        let action = match pending {
            Pending::Mulligan { .. } => PlayerAction::MulliganKeep,
            Pending::Priority { .. } if seat == PlayerId::new(1) => PlayerAction::PassPriority,
            _ => agent.act_with_context(&view, pending, &engine.decision_context()),
        };
        if matches!(pending, Pending::ChooseNumber { .. }) {
            assert_eq!(action, PlayerAction::ChooseNumber(2));
            chosen_x = true;
        }
        if matches!(pending, Pending::ChoosePlayer { .. }) {
            assert_eq!(
                action,
                PlayerAction::ChoosePlayer(PlayerId::new(0)),
                "draw effects belong to the caster"
            );
        }
        engine
            .apply(seat, action)
            .expect("X mana and target are legal");
    }
    panic!("the two-card draw must resolve");
}

/// The fuzzer's panic (2026-09-29): a house seat with twenty mana, Lose
/// Focus in hand and a hostile spell on the stack.
///
/// Lose Focus is `{1}{U}` with replicate `{U}`, payable "any number of
/// times" (CR 702.56a). The agent priced its replicate payments from the
/// most its budget allows down, building each price before asking whether
/// it fit, and a cost held sixteen symbols: at a budget of fifteen or more
/// the first price, `{1}{U}` and fifteen `{U}`, asserted inside
/// `HeuristicAgent::act`, and the hosted game went down with it. Priced as
/// one counted cost, twenty mana is eighteen payments: the agent floats all
/// twenty, casts, and takes every payment the engine offers.
///
/// Seat 1 casts a Dark Ritual in seat 0's upkeep, and seat 0 passes by hand
/// until then, so the agent is first asked with the Ritual on the stack, as
/// the fuzzer's house was.
#[test]
fn a_house_seat_with_twenty_mana_replicates_lose_focus_eighteen_times() {
    let (me, them) = (PlayerId::new(0), PlayerId::new(1));
    let mut preset = position(&["Lose Focus"], &["Island"; 20]);
    preset.seats[1].starting_hand = Some(vec![entry("Dark Ritual")]);
    preset.seats[1].starting_battlefield = vec![entry("Swamp")];
    let mut engine = Engine::new(&preset, RegistryLookup).unwrap();
    let agent = HeuristicAgent::new(AIProfile::EXPERT);
    let mut replicated = None;
    for seq in 0..200 {
        let pending = engine.pending().clone();
        let seat = pending_player(&pending).expect("nobody has lost");
        if replicated.is_some() && matches!(pending, Pending::Priority { .. }) {
            break;
        }
        let view = asked_view(engine.state(), seat, seq, &pending);
        let action = match &pending {
            Pending::Mulligan { .. } => PlayerAction::MulliganKeep,
            Pending::Priority { legal, .. } if seat == them => {
                let ritual = entry("Dark Ritual").card;
                if let Some(&card) = legal.castable.first() {
                    PlayerAction::CastSpell { card }
                } else if view.hand.iter().any(|c| c.card.index == ritual)
                    && let Some(&source) = legal.mana_abilities.first()
                {
                    PlayerAction::ActivateManaAbility { source }
                } else {
                    PlayerAction::PassPriority
                }
            }
            _ if seat == them => panic!("unexpected opposing question: {pending:?}"),
            Pending::Priority { .. } if view.stack.is_empty() => PlayerAction::PassPriority,
            _ => agent.act_with_context(&view, &pending, &engine.decision_context()),
        };
        if let Pending::ChooseNumber {
            max,
            reason: baylee_engine::choice::NumberPrompt::Replicate { .. },
            ..
        } = &pending
        {
            assert_eq!(*max, 18, "twenty mana pays {{1}}{{U}} and eighteen {{U}}");
            replicated = Some(action.clone());
        }
        engine.apply(seat, action).expect("a legal answer");
    }
    assert_eq!(
        replicated,
        Some(PlayerAction::ChooseNumber(18)),
        "the house takes every payment the engine offers"
    );
    assert_eq!(
        engine.state().players[usize::from(me.get())]
            .mana_pool
            .total(),
        0,
        "and pays all twenty"
    );
}

/// Plays seat 0's first main phase out with the agent, the other seat
/// passing, until `done` holds or a hundred questions pass. Returns what the
/// agent answered to the kicker question, if it was asked.
fn play_the_kicker(
    hand: &[&str],
    board: &[&str],
    done: impl Fn(&PlayerView) -> bool,
) -> (Option<PlayerAction>, PlayerView) {
    let preset = position(hand, board);
    let mut engine = Engine::new(&preset, RegistryLookup).unwrap();
    let agent = HeuristicAgent::new(AIProfile::EXPERT);
    let mut kicker = None;
    for seq in 0..100 {
        let pending = engine.pending();
        let seat = pending_player(pending).unwrap();
        let view = asked_view(engine.state(), seat, seq, pending);
        if seat == PlayerId::new(0) && done(&view) {
            return (kicker, view);
        }
        let action = match pending {
            Pending::Mulligan { .. } => PlayerAction::MulliganKeep,
            Pending::Priority { .. } if seat == PlayerId::new(1) => PlayerAction::PassPriority,
            _ => agent.act_with_context(&view, pending, &engine.decision_context()),
        };
        if let Pending::YesNo {
            prompt: baylee_engine::choice::YesNoPrompt::Kicker,
            ..
        } = pending
        {
            kicker = Some(action.clone());
        }
        engine.apply(seat, action).expect("the kicked cast is paid");
    }
    panic!("the spell never resolved; kicker answered {kicker:?}");
}

/// Kicker (CR 702.33a) and "you may waterbend" (CR 701.67a) through the real
/// engine, which pays the kicked total out of the floating pool alone: the
/// agent has to float it before the cast, answer yes, and — for waterbend —
/// tap its creatures toward the {6} when the convoke question comes.
#[test]
fn a_kicker_and_a_waterbend_are_paid_when_the_mana_is_there() {
    let islands = ["Island"; 9];
    let mut board: Vec<&str> = islands.to_vec();
    board.push("Ondu Cleric");
    // A copy token carries neither `card` nor `token` in the view, so the
    // copies are counted by name.
    let clerics = |v: &PlayerView| {
        v.battlefield_of(v.seat)
            .filter(|o| o.name == "Ondu Cleric")
            .count()
    };
    let (kicker, view) = play_the_kicker(&["Rite of Replication"], &board, |v| clerics(v) > 1);
    assert_eq!(kicker, Some(PlayerAction::YesNo(true)));
    assert_eq!(
        clerics(&view),
        6,
        "kicked, Rite of Replication makes five copies"
    );

    let mut board = vec!["Island"; 3];
    board.extend(["Ondu Cleric"; 6]);
    let (kicker, view) = play_the_kicker(&["Spirit Water Revival"], &board, |v| v.hand.len() > 2);
    assert_eq!(kicker, Some(PlayerAction::YesNo(true)));
    assert_eq!(
        view.hand.len(),
        7,
        "three Islands and six creatures pay the waterbend, so it draws seven"
    );
    assert!(view.seats[0].no_max_hand_size);
}

#[test]
fn spirit_water_revivals_permanent_hand_limit_status_reaches_every_seat() {
    let mut board = vec!["Island"; 3];
    board.extend(["Ondu Cleric"; 6]);
    let mut engine =
        Engine::new(&position(&["Spirit Water Revival"], &board), RegistryLookup).unwrap();
    let agent = HeuristicAgent::new(AIProfile::EXPERT);
    for seq in 0..100 {
        let pending = engine.pending();
        let seat = pending_player(pending).unwrap();
        let view = asked_view(engine.state(), seat, seq, pending);
        if view.seats[0].no_max_hand_size {
            assert!(
                view.exile[0]
                    .iter()
                    .any(|o| o.name == "Spirit Water Revival")
            );
            for viewer in 0..2 {
                let shown = asked_view(engine.state(), PlayerId::new(viewer), seq, pending);
                assert!(shown.seats[0].no_max_hand_size);
                assert!(!shown.seats[1].no_max_hand_size);
            }
            return;
        }
        let action = match pending {
            Pending::Mulligan { .. } => PlayerAction::MulliganKeep,
            Pending::Priority { .. } if seat == PlayerId::new(1) => PlayerAction::PassPriority,
            _ => agent.act_with_context(&view, pending, &engine.decision_context()),
        };
        engine.apply(seat, action).unwrap();
    }
    panic!("Spirit Water Revival never granted its continuing effect");
}

/// #224 through the real engine: Chord of Calling for X = 1 off four floating
/// Forests leaves nothing for a convoke tap to pay, so both Clerics stay
/// untapped. The engine asks the tap question (its `max` is the `{1}`,
/// #229), and the agent used to answer it with as many as it allowed.
#[test]
fn a_convoke_cast_the_pool_already_pays_taps_no_creature() {
    let mut board = vec!["Forest"; 4];
    board.extend(["Ondu Cleric"; 2]);
    let mut engine = Engine::new(&position(&["Chord of Calling"], &board), RegistryLookup).unwrap();
    let agent = HeuristicAgent::new(AIProfile::EXPERT);
    let mut answered = None;
    for seq in 0..100 {
        let pending = engine.pending();
        let seat = pending_player(pending).unwrap();
        let view = asked_view(engine.state(), seat, seq, pending);
        if seat == PlayerId::new(0) && view.hand.is_empty() && view.stack.is_empty() {
            let (max, objects) = answered.expect("Chord was cast without a tap question");
            assert_eq!(max, 1, "the question is bounded by Chord's {{1}}");
            assert_eq!(
                objects,
                Vec::<ObjectId>::new(),
                "a Cleric was tapped for mana already floating"
            );
            let tapped = view
                .battlefield_of(view.seat)
                .filter(|o| o.name == "Ondu Cleric")
                .filter(|o| o.status.contains(baylee_view::ObjectStatus::TAPPED))
                .count();
            assert_eq!(tapped, 0, "a Cleric was left tapped");
            return;
        }
        let action = match pending {
            Pending::Mulligan { .. } => PlayerAction::MulliganKeep,
            Pending::Priority { .. } if seat == PlayerId::new(1) => PlayerAction::PassPriority,
            _ => agent.act_with_context(&view, pending, &engine.decision_context()),
        };
        if let (
            Pending::ChooseTargets {
                max,
                reason: baylee_engine::choice::TargetPrompt::Convoke,
                ..
            },
            PlayerAction::ChooseTargets { objects, .. },
        ) = (pending, &action)
        {
            answered = Some((*max, objects.clone()));
        }
        engine
            .apply(seat, action)
            .expect("the agent's answer is legal");
    }
    panic!("Chord of Calling never resolved");
}
