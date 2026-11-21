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
            prompt:
                baylee_engine::choice::YesNoPrompt::Kicker
                | baylee_engine::choice::YesNoPrompt::Bargain
                | baylee_engine::choice::YesNoPrompt::KickerOption { .. }
                | baylee_engine::choice::YesNoPrompt::Buyback { .. },
            ..
        } = pending
        {
            kicker = Some(action.clone());
        }
        engine.apply(seat, action).expect("the kicked cast is paid");
    }
    panic!("the spell never resolved; kicker answered {kicker:?}");
}

/// Buyback (CR 702.27a) through the real engine: with five Swamps and nothing
/// else to do with them the agent floats {B} and {4}, answers yes, and has
/// its Lab Rats back with a Rat on the table. A sixth land is not needed.
#[test]
fn a_house_seat_with_the_mana_to_spare_buys_its_spell_back() {
    let board = ["Swamp"; 5];
    let (paid, view) = play_the_kicker(&["Lab Rats"], &board, |v| {
        v.battlefield_of(v.seat).any(|o| o.name == "Rat")
    });
    assert_eq!(paid, Some(PlayerAction::YesNo(true)));
    assert_eq!(view.hand.len(), 1, "the Rats came home");
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

/// Goblin Warchief: "Goblin spells you cast cost {1} less to cast." The
/// engine charges the Sky Raider `{1}{R}` under it, and the house priced the
/// printed `{2}{R}`: with two Mountains it held the Goblin for a land it did
/// not have (NEW-03, 3.11.2026; CR 601.2f). The cast has to come on the first
/// turn, before a drawn land would pay the printed price.
#[test]
fn the_ai_casts_a_goblin_for_the_price_the_warchief_leaves() {
    let me = PlayerId::new(0);
    let preset = position(
        &["Goblin Sky Raider"],
        &["Goblin Warchief", "Mountain", "Mountain"],
    );
    let mut engine = Engine::new(&preset, RegistryLookup).unwrap();
    let agent = HeuristicAgent::new(AIProfile::EXPERT);
    for seq in 0..100 {
        let pending = engine.pending();
        let seat = pending_player(pending).expect("fixture remains live");
        let view = asked_view(engine.state(), seat, seq, pending);
        if view.battlefield_of(me).any(|o| {
            o.card
                .is_some_and(|c| c.index == entry("Goblin Sky Raider").card)
        }) {
            assert_eq!(view.turn, 1, "cast off the two Mountains, not a later land");
            assert!(untapped(&view, "Mountain").is_empty());
            return;
        }
        assert_eq!(view.turn, 1, "the house passed its turn holding the Goblin");
        let action = match pending {
            Pending::Mulligan { .. } => PlayerAction::MulliganKeep,
            Pending::Priority { .. } if seat != me => PlayerAction::PassPriority,
            _ => agent.act_with_context(&view, pending, &engine.decision_context()),
        };
        engine
            .apply(seat, action)
            .expect("the house answers from its offer");
    }
    panic!("the table never moved past the first turn");
}

/// A spell that has to target and has nothing to point at is not cast, and
/// so nothing is tapped for it (CR 601.2c). Terror, Fear (an Aura's "enchant
/// creature" is its target, CR 303.4a) and Disenchant over boards that hold
/// no legal target for them: the house's first turn ends with every land it
/// had still untapped. NEW-04, 3.11.2026.
#[test]
fn the_ai_taps_nothing_for_a_spell_with_no_legal_target() {
    let me = PlayerId::new(0);
    for (spell, lands, theirs) in [
        ("Terror", &["Swamp", "Swamp"][..], &[][..]),
        // Terror says "nonblack": a black creature is no target for it.
        ("Terror", &["Swamp", "Swamp"][..], &["Vampire Bats"][..]),
        ("Fear", &["Swamp", "Swamp"][..], &[][..]),
        (
            "Disenchant",
            &["Plains", "Plains"][..],
            &["Grizzly Bears"][..],
        ),
    ] {
        let mut preset = position(&[spell], lands);
        preset.seats[1].starting_battlefield = theirs.iter().map(|name| entry(name)).collect();
        let mut engine = Engine::new(&preset, RegistryLookup).unwrap();
        let agent = HeuristicAgent::new(AIProfile::EXPERT);
        for seq in 0..100 {
            let pending = engine.pending();
            let seat = pending_player(pending).expect("fixture remains live");
            let view = asked_view(engine.state(), seat, seq, pending);
            if view.turn > 1 {
                break;
            }
            let tapped = view
                .battlefield_of(me)
                .filter(|o| o.status.contains(baylee_view::ObjectStatus::TAPPED))
                .count();
            assert_eq!(
                tapped, 0,
                "the house tapped a land for {spell} with nothing to target ({theirs:?})"
            );
            let action = match pending {
                Pending::Mulligan { .. } => PlayerAction::MulliganKeep,
                Pending::Priority { .. } if seat != me => PlayerAction::PassPriority,
                _ => agent.act_with_context(&view, pending, &engine.decision_context()),
            };
            engine
                .apply(seat, action)
                .expect("the house answers from its offer");
        }
    }
}

/// Under Urborg, Tomb of Yawgmoth and Yavimaya, Cradle of Growth each of the
/// two is a Swamp Forest with no printed mana ability: it taps for `{B}` or
/// `{G}` through CR 305.6 alone. The house read that shortcut only for a land
/// of one basic type, so it saw no source and held a Grizzly Bears it could
/// cast (NEW-09, 3.11.2026).
#[test]
fn the_ai_taps_a_land_with_two_basic_types_for_the_colour_it_needs() {
    let me = PlayerId::new(0);
    let preset = position(
        &["Grizzly Bears"],
        &["Urborg, Tomb of Yawgmoth", "Yavimaya, Cradle of Growth"],
    );
    let mut engine = Engine::new(&preset, RegistryLookup).unwrap();
    let agent = HeuristicAgent::new(AIProfile::EXPERT);
    for seq in 0..100 {
        let pending = engine.pending();
        let seat = pending_player(pending).expect("fixture remains live");
        let view = asked_view(engine.state(), seat, seq, pending);
        if view.battlefield_of(me).any(|o| {
            o.card
                .is_some_and(|c| c.index == entry("Grizzly Bears").card)
        }) {
            assert_eq!(view.turn, 1);
            return;
        }
        assert_eq!(view.turn, 1, "the house passed its turn holding the Bears");
        let action = match pending {
            Pending::Mulligan { .. } => PlayerAction::MulliganKeep,
            Pending::Priority { .. } if seat != me => PlayerAction::PassPriority,
            _ => agent.act_with_context(&view, pending, &engine.decision_context()),
        };
        engine
            .apply(seat, action)
            .expect("the house answers from its offer");
    }
    panic!("the table never moved past the first turn");
}

fn pass_through_step(
    engine: &mut Engine<RegistryLookup>,
    agent: &HeuristicAgent,
    me: PlayerId,
    allow_agent_on_me_main: bool,
    stop_when: impl Fn(&Engine<RegistryLookup>, &PlayerView) -> bool,
) {
    for seq in 0..300 {
        let pending = engine.pending().clone();
        let Some(seat) = pending_player(&pending) else {
            return;
        };
        let view = asked_view(engine.state(), seat, seq, &pending);
        if matches!(&pending, Pending::Priority { .. }) && stop_when(engine, &view) {
            return;
        }
        let action = match &pending {
            Pending::Mulligan { .. } => PlayerAction::MulliganKeep,
            Pending::ChooseAttackers { .. } => PlayerAction::DeclareAttackers { attackers: vec![] },
            Pending::ChooseBlockers { .. } => PlayerAction::DeclareBlockers { blockers: vec![] },
            Pending::DiscardChoice { count, .. } => PlayerAction::ChooseObjects {
                objects: view
                    .hand
                    .iter()
                    .take(*count as usize)
                    .map(|o| o.id)
                    .collect(),
            },
            Pending::Priority { .. } if seat != me || !allow_agent_on_me_main => {
                PlayerAction::PassPriority
            }
            _ => agent.act_with_context(&view, &pending, &engine.decision_context()),
        };
        engine.apply(seat, action).expect("legal game step");
    }
    panic!("timed out waiting for expected board state");
}

fn find_on_board(view: &PlayerView, owner: PlayerId, name: &str) -> Option<ObjectId> {
    let idx = entry(name).card;
    view.battlefield_of(owner)
        .find(|o| o.card.is_some_and(|c| c.index == idx))
        .map(|o| o.id)
}

fn murder_commander_and_recast(
    engine: &mut Engine<RegistryLookup>,
    agent: &HeuristicAgent,
    me: PlayerId,
    them: PlayerId,
    target_name: &str,
) {
    pass_through_step(engine, agent, me, false, |_, v| {
        v.active == them && v.phase == baylee_view::Phase::FirstMain && v.stack.is_empty()
    });
    let view_them = asked_view(engine.state(), them, 0, engine.pending());
    let target_id = find_on_board(&view_them, me, target_name).expect("commander on battlefield");
    for swamp in view_them
        .battlefield_of(them)
        .filter(|o| !o.status.contains(baylee_view::ObjectStatus::TAPPED))
        .map(|o| o.id)
        .collect::<Vec<_>>()
    {
        engine
            .apply(them, PlayerAction::ActivateManaAbility { source: swamp })
            .unwrap();
    }
    let murder = view_them
        .hand
        .iter()
        .find(|c| c.name == "Murder")
        .expect("Murder in seat 1 hand")
        .id;
    engine
        .apply(them, PlayerAction::CastSpell { card: murder })
        .unwrap();
    engine
        .apply(
            them,
            PlayerAction::ChooseObjects {
                objects: vec![target_id],
            },
        )
        .unwrap();
    pass_through_step(engine, agent, me, true, |e, v| {
        v.active == me
            && v.turn > 1
            && v.stack.is_empty()
            && v.battlefield_of(me).filter(|o| o.commander).count() == 2
            && e.state().commanders[0]
                .first()
                .is_some_and(|c| c.casts == 2)
    });
}

fn drive_commander_damage_race(
    engine: &mut Engine<RegistryLookup>,
    me: PlayerId,
    them: PlayerId,
    trynn: &str,
    silvar: &str,
) {
    let (mut trynn_hit, mut checked_split) = (false, false);
    for seq in 0..600 {
        let pending = engine.pending().clone();
        if let Pending::GameOver(res) = pending {
            assert_eq!(res.winner, Some(baylee_engine::win::Victor::Player(me)));
            assert!(
                checked_split,
                "must observe combined >= 21 before single reaches 21"
            );
            assert!(
                engine.state().players[1].life > 0,
                "40 life proves commander damage won"
            );
            return;
        }
        let seat = pending_player(&pending).expect("live game");
        let view = asked_view(engine.state(), seat, seq, &pending);
        if !checked_split && view.phase == baylee_view::Phase::SecondMain && trynn_hit {
            let tally = &engine.state().players[1].commander_damage;
            let sum: u16 = tally.iter().map(|(_, d)| *d).sum();
            let max = tally.iter().map(|(_, d)| *d).max().unwrap_or(0);
            if max == 20 {
                assert!(
                    sum >= 23,
                    "combined partner commander damage is {sum} >= 21"
                );
                checked_split = true;
            }
        }
        let action = match &pending {
            Pending::ChooseAttackers { attackers, .. } => {
                let mut swing = Vec::new();
                if let Some(s) = find_on_board(&view, me, silvar)
                    && attackers.contains(&s)
                {
                    swing.push((s, baylee_core::ids::Defender::Player(them)));
                }
                if !trynn_hit
                    && let Some(t) = find_on_board(&view, me, trynn)
                    && attackers.contains(&t)
                {
                    swing.push((t, baylee_core::ids::Defender::Player(them)));
                    trynn_hit = true;
                }
                PlayerAction::DeclareAttackers { attackers: swing }
            }
            Pending::ChooseBlockers { .. } => PlayerAction::DeclareBlockers { blockers: vec![] },
            Pending::DiscardChoice { count, .. } => PlayerAction::ChooseObjects {
                objects: view
                    .hand
                    .iter()
                    .take(*count as usize)
                    .map(|o| o.id)
                    .collect(),
            },
            Pending::Priority { .. } => PlayerAction::PassPriority,
            other => panic!("unexpected pending in combat race: {other:?}"),
        };
        engine
            .apply(seat, action)
            .expect("legal combat progression");
    }
    panic!("Silvar never dealt fatal 21 commander damage");
}

#[test]
fn partner_with_commanders_keep_separate_tax_and_separate_twenty_one_damage() {
    let (me, them) = (PlayerId::new(0), PlayerId::new(1));
    let (trynn, silvar) = ("Trynn, Champion of Freedom", "Silvar, Devourer of the Free");
    let trynn_leader = baylee_cards::decks::leader_of(entry(trynn).card).unwrap();
    let silvar_leader = baylee_cards::decks::leader_of(entry(silvar).card).unwrap();
    assert!(baylee_cards::decks::may_lead_together(
        &trynn_leader,
        &silvar_leader
    ));

    let mut preset = position(
        &[],
        &[
            "Plains", "Plains", "Plains", "Plains", "Swamp", "Swamp", "Swamp", "Swamp", "Mountain",
            "Mountain", "Mountain",
        ],
    );
    for s in &mut preset.seats {
        s.starting_life = Some(40);
    }
    preset.seats[0].commanders = vec![entry(trynn), entry(silvar)];
    preset.seats[1].starting_battlefield = vec![entry("Swamp"), entry("Swamp"), entry("Swamp")];
    preset.seats[1].starting_hand = Some(vec![entry("Murder")]);

    let mut engine = Engine::new(&preset, RegistryLookup).unwrap();
    let agent = HeuristicAgent::new(AIProfile::EXPERT);
    pass_through_step(&mut engine, &agent, me, true, |e, v| {
        v.stack.is_empty()
            && v.battlefield_of(me).filter(|o| o.commander).count() == 2
            && e.state().commanders[0]
                .iter()
                .map(|c| c.casts)
                .collect::<Vec<_>>()
                == vec![1, 1]
    });
    murder_commander_and_recast(&mut engine, &agent, me, them, trynn);
    let casts: Vec<u32> = engine.state().commanders[0]
        .iter()
        .map(|c| c.casts)
        .collect();
    assert_eq!(
        casts,
        vec![2, 1],
        "tax tracks each partner commander independently"
    );
    drive_commander_damage_race(&mut engine, me, them, trynn, silvar);
}

#[test]
fn choose_a_background_deploys_both_commanders_with_independent_cast_counts() {
    let me = PlayerId::new(0);
    let (jaheira, bg) = ("Jaheira, Friend of the Forest", "Far Traveler");
    let jl = baylee_cards::decks::leader_of(entry(jaheira).card).unwrap();
    let bl = baylee_cards::decks::leader_of(entry(bg).card).unwrap();
    assert!(bl.eligible && bl.background);
    assert!(baylee_cards::decks::may_lead_together(&jl, &bl));

    let mut preset = position(
        &[],
        &["Forest", "Forest", "Forest", "Plains", "Plains", "Plains"],
    );
    preset.seats[0].commanders = vec![entry(jaheira), entry(bg)];
    let mut engine = Engine::new(&preset, RegistryLookup).unwrap();
    let agent = HeuristicAgent::new(AIProfile::EXPERT);
    pass_through_step(&mut engine, &agent, me, true, |_, v| {
        v.stack.is_empty() && v.battlefield_of(me).filter(|o| o.commander).count() == 2
    });
    let casts: Vec<u32> = engine.state().commanders[0]
        .iter()
        .map(|c| c.casts)
        .collect();
    assert_eq!(casts, vec![1, 1]);
}
