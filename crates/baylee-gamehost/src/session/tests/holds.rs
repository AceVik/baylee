use super::*;

/// A session starts, the human answers mulligans, and the AI seat is
/// driven automatically between human choices.
#[test]
fn session_pumps_ai_between_human_choices() {
    let mut session = Session::new(&test_preset()).expect("session builds");
    let human = session.human_seats()[0];
    let mut human_choices = 0;
    for _ in 0..50 {
        let envelopes = session.pump();
        if envelopes.is_empty() {
            break;
        }
        for (_, env) in envelopes {
            let Some(v1::envelope::Msg::ChoiceRequest(req)) = env.msg else {
                continue;
            };
            let pending: Pending = serde_json::from_slice(&req.pending_json).unwrap();
            human_choices += 1;
            let action = match pending {
                Pending::Mulligan { .. } => PlayerAction::MulliganKeep,
                Pending::ChooseAttackers { .. } => {
                    PlayerAction::DeclareAttackers { attackers: vec![] }
                }
                _ => PlayerAction::PassPriority,
            };
            let _ = session.act(human, action);
        }
    }
    assert!(human_choices > 0, "the human received choices");
}

/// A priority hold is a statement about what its owner intends to respond
/// to, which makes it exactly the kind of read a player is entitled to
/// keep. It reaches that seat's own view and nobody else's.
///
/// It also has to reach the view *at all*: the hold lives in the engine's
/// `SeatAutomation` and not in the `GameState` the view is built from, so
/// there is a parameter carrying it across and nothing but a test says it
/// was filled in.
#[test]
fn a_seat_sees_its_own_hold_and_not_the_other_seats() {
    // Both seats human, so both are sent a view and the second half of
    // this test has something to read.
    let mut preset = test_preset();
    preset.seats[1].controller = SeatController::Open;
    let mut session = Session::new(&preset).expect("session builds");
    let seat = PlayerId::new(0);
    let turn = seat_view(&session, seat).turn;
    assert!(!seat_view(&session, seat).priority_held, "nothing held yet");

    // Set while the opening mulligan is pending, which is the point: the
    // engine takes an automation setting from any seated player whether or
    // not it is that player's decision, and without that a hold could
    // never be cancelled.
    let routed = session
        .act(
            seat,
            PlayerAction::SetPriorityHold(baylee_engine::choice::PriorityHold::UntilEndOfTurn {
                turn,
            }),
        )
        .expect("a seat may state a standing order at any time");

    assert!(
        routed_view(&routed, seat).priority_held,
        "the seat that set the hold was never sent a view saying so"
    );
    assert!(
        !routed_view(&routed, PlayerId::new(1)).priority_held,
        "one seat's standing order is not the other's to read"
    );
}

/// What a seat's policies answered for it stays in its view until the
/// seat next answers by hand, and is counted over the whole game (#234).
///
/// The window is cleared before the journal is read: the answer that
/// clears it can run on into the policy answering again, and that answer
/// is news. Here a keep starts the game and the same apply reaches the
/// first upkeep, where the seat's yield passes over its own arena.
#[test]
fn a_seats_own_answer_clears_what_its_policies_answered() {
    let arena = baylee_cards::by_oracle_id("ee579a32-a048-4335-b966-231ba731cdea")
        .expect("Phyrexian Arena is in the pool")
        .index;
    let mut preset = test_preset();
    preset.seats[1].controller = SeatController::Open;
    preset.seats[0].starting_battlefield = vec![DeckEntry {
        card: arena,
        print: PrintRef::new(0),
    }];
    let mut session = Session::new(&preset).expect("session builds");
    let (me, them) = (PlayerId::new(0), PlayerId::new(1));
    let trigger = baylee_core::ids::AbilityRef::new(arena, 0);
    let acts = |routed: &[(PlayerId, Envelope)], seat| -> Vec<(u32, PolicyAnswer)> {
        routed_view(routed, seat)
            .policy_acts
            .iter()
            .map(|act| (act.number, act.answer))
            .collect()
    };
    let _ = session.pump();
    session
        .act(
            me,
            PlayerAction::SetAbilityPolicy {
                ability: trigger,
                pass: true,
                answer: None,
            },
        )
        .expect("a seat may set a policy at any time");
    session
        .act(them, PlayerAction::MulliganKeep)
        .expect("the other seat keeps");

    let routed = session
        .act(me, PlayerAction::MulliganKeep)
        .expect("this seat keeps");
    assert_eq!(acts(&routed, me), [(1, PolicyAnswer::Passed)]);
    let named = routed_view(&routed, me).policy_acts[0].ability;
    assert_eq!(named.ability, Some(trigger));
    assert!(named.text.is_some(), "named in full while on the stack");
    assert_eq!(acts(&routed, them), [], "one seat's policy is its own");

    let routed = session
        .act(them, PlayerAction::PassPriority)
        .expect("the other seat lets the trigger resolve");
    assert_eq!(
        acts(&routed, me),
        [(1, PolicyAnswer::Passed)],
        "another seat's answer is not this seat's"
    );
    let routed = session
        .act(me, PlayerAction::PassPriority)
        .expect("this seat passes its upkeep by hand");
    assert_eq!(acts(&routed, me), []);

    // Round to this seat's next upkeep, as passively as a player can.
    let mut next = None;
    for _ in 0..200 {
        let (player, action) = match session.engine.pending().clone() {
            Pending::Priority { player, .. } => (player, PlayerAction::PassPriority),
            Pending::DiscardChoice { player, count } => {
                let hand = session
                    .engine
                    .state()
                    .zones
                    .list(ZoneLocation::Hand(player));
                let objects = hand.iter().take(usize::from(count)).copied().collect();
                (player, PlayerAction::ChooseObjects { objects })
            }
            Pending::ChooseAttackers { player, .. } => {
                (player, PlayerAction::DeclareAttackers { attackers: vec![] })
            }
            other => panic!("an Island deck was asked {other:?}"),
        };
        let routed = session.act(player, action).expect("a passive answer");
        if !acts(&routed, me).is_empty() {
            next = Some(routed);
            break;
        }
    }
    let routed = next.expect("this seat's next upkeep came round");
    assert_eq!(
        acts(&routed, me),
        [(2, PolicyAnswer::Passed)],
        "numbered on over the game, not from the window"
    );
}

/// The view names the seat the table is waiting for, which is a wider
/// question than who holds priority — and the field answered the narrow
/// one until `VIEW_VERSION` 23.
///
/// Priority (CR 117) exists only while the engine is offering it, so a
/// seat taking a mulligan, picking blockers or discarding to hand size
/// holds none and was reported as nobody. Its three readers — the stack
/// head's "waiting for", the seat caret and the board model's pod — all
/// mean "waiting on them", so all three went blank on every question that
/// was not a priority pass. `pending_player` is the question they were
/// asking.
///
/// The opponent's copy is the half that cannot be worked out client-side:
/// a session sends the pending question only to the seat it is addressed
/// to, so the other seat has nothing else to read it off.
///
/// The opening mulligans have no such other seat: every seat is asked
/// its own at once (#257), so each seat's view names itself until it has
/// kept, and nobody after.
#[test]
fn a_seat_that_holds_no_priority_is_still_the_seat_being_waited_for() {
    // Both seats human, so the seat that is not being asked is sent a
    // view and the assertions on it have something to read.
    let mut preset = test_preset();
    preset.seats[1].controller = SeatController::Open;
    let mut session = Session::new(&preset).expect("session builds");
    let (zero, one) = (PlayerId::new(0), PlayerId::new(1));

    assert!(
        matches!(session.pending(), Pending::Mulligan { .. }),
        "the game opens on a question nobody holds priority for"
    );
    for seat in [zero, one] {
        assert_eq!(
            seat_view(&session, seat).awaiting,
            Some(seat),
            "a seat deciding its mulligan is told the table is waiting \
             for it"
        );
    }
    session
        .act(zero, PlayerAction::MulliganKeep)
        .expect("a keep");
    assert_eq!(
        seat_view(&session, zero).awaiting,
        None,
        "a seat that has kept is asked nothing"
    );
    assert_eq!(seat_view(&session, one).awaiting, Some(one));
    session
        .act(one, PlayerAction::MulliganKeep)
        .expect("a keep");

    // Play the game out with the house agent answering both chairs, and
    // hold both views against the seat that actually owes an answer.
    // Bounded on the questions seen rather than on the loop, because a
    // run that stopped early would assert almost nothing.
    let agent = HeuristicAgent::new(AIProfile::default());
    let mut priority_questions = 0usize;
    let mut other_questions = 0usize;
    for _ in 0..600 {
        let Some(seat) = session.awaiting_seat() else {
            break;
        };
        if matches!(session.pending(), Pending::Priority { .. }) {
            priority_questions += 1;
        } else {
            other_questions += 1;
        }
        for viewer in [zero, one] {
            assert_eq!(
                seat_view(&session, viewer).awaiting,
                Some(seat),
                "every question names the seat that owes the answer, in \
                 both views"
            );
        }
        let view = seat_view(&session, seat);
        let action = agent.act(&view, session.pending());
        if session.act(seat, action).is_err() {
            break;
        }
    }
    assert!(
        priority_questions > 10,
        "the game reached priority repeatedly: {priority_questions}"
    );
    assert!(
        other_questions > 1,
        "and asked questions that were not priority: {other_questions}"
    );
}

/// The one hold that answers rather than withholds does not light the
/// indicator, because it never keeps a decision from being offered.
#[test]
fn passing_when_there_is_nothing_to_do_is_not_a_hold() {
    let mut session = Session::new(&test_preset()).expect("session builds");
    let seat = PlayerId::new(0);
    session
        .act(
            seat,
            PlayerAction::SetPriorityHold(baylee_engine::choice::PriorityHold::PassWhenNothingToDo),
        )
        .expect("a seat may state a standing order at any time");
    assert!(
        !seat_view(&session, seat).priority_held,
        "it fires only where passing was the sole legal action, so a seat \
         running it is never actually being kept from a decision"
    );
}

/// A table where seat 0 can cast Swords to Plowshares at seat 1's Roaming
/// Throne, which prints ward {2}, and has two Plains left over to pay it.
fn a_table_with_a_warded_creature() -> GamePreset {
    let named = |name: &str| DeckEntry {
        card: baylee_cards::decks::by_name(name).expect("a card this pool compiles"),
        print: PrintRef::new(0),
    };
    let mut preset = test_preset();
    preset.seats[1].controller = SeatController::Open;
    // Three Plains: one pays for the spell, two answer the tax. A seat
    // that could not pay at all would be inside a different question.
    preset.seats[0].starting_hand = Some(vec![named("Swords to Plowshares")]);
    preset.seats[0].starting_battlefield = vec![named("Plains"); 3];
    preset.seats[1].starting_hand = Some(vec![]);
    preset.seats[1].starting_battlefield = vec![named("Roaming Throne")];
    preset
}

/// Plays the table into ward's CR 605.3a window: cast the removal at the
/// warded creature and agree to pay. Answers whether the tax was ever
/// offered, so a fixture that stopped somewhere else cannot pass quietly.
fn drive_into_ward_s_payment_window(session: &mut Session, payer: PlayerId) -> bool {
    // Cast the removal at the warded creature and agree to pay the tax,
    // taking every answer from the engine's own offer. The tap before the
    // cast is not scene-setting: `can_cast` probes the *pool* and not the
    // untapped lands (`casting::affordable`), so a spell is castable only
    // once its mana is floating — which is the same rule the payment
    // window exists to serve.
    let mut agreed = false;
    let mut cast = false;
    for _ in 0..200 {
        if session.engine.payment_window().is_some() {
            break;
        }
        let Some(seat) = session.awaiting_seat() else {
            break;
        };
        let action = match session.pending() {
            Pending::Mulligan { .. } => PlayerAction::MulliganKeep,
            // The Throne's own "as this enters, choose a creature type",
            // which seat 1 answers before anything else can happen.
            Pending::ChooseSubtype { options, .. } => PlayerAction::ChooseSubtype(options[0]),
            Pending::ChooseTargets { options, .. } => PlayerAction::ChooseTargets {
                objects: options.first().copied().into_iter().collect(),
                players: vec![],
            },
            Pending::YesNo { .. } => {
                // The resolution is already suspended with the price
                // while this question is being asked, and no window is
                // open yet. A seat being asked whether it *wants* a debt
                // does not have one, so nothing is owed here.
                assert_eq!(
                    seat_view(session, payer).owed,
                    None,
                    "a seat still deciding whether to pay owes nothing yet"
                );
                agreed = true;
                PlayerAction::YesNo(true)
            }
            // Exactly one Plains is tapped, and only to make the spell
            // castable. Tapping the other two here would settle the tax
            // out of the pool and open no window at all, which is the
            // engine's own "nothing to press" branch and a different
            // game from this one.
            Pending::Priority { legal, .. } if seat == payer && !cast => {
                if let Some(&card) = legal.castable.first() {
                    cast = true;
                    PlayerAction::CastSpell { card }
                } else if let Some(&source) = legal.mana_abilities.first() {
                    PlayerAction::ActivateManaAbility { source }
                } else {
                    PlayerAction::PassPriority
                }
            }
            _ => PlayerAction::PassPriority,
        };
        if session.act(seat, action).is_err() {
            break;
        }
    }
    agreed
}

/// The seat inside a CR 605.3a payment window is told what it owes, and
/// so is the seat watching it.
///
/// The window is an ordinary `Pending::Priority` offering mana abilities
/// and nothing else — deliberately, so that a client draws it and an
/// agent answers it with no new question shape — which is exactly why
/// nothing could tell it apart from a quiet priority pass with no plays.
/// The house agent said yes to ward's tax, was handed the window, saw
/// nothing castable over two untapped Plains, passed, and lost its own
/// spell. `crates/baylee-gamehost/tests/ai_ward.rs` on the `ai` branch
/// pins that from the agent's side; this is the half the view owes it.
///
/// Asserted from the **bystander's** view as well as the payer's,
/// because that is the vantage point where a missing field is visible:
/// the pending question is sent only to the seat it is addressed to, so
/// seat 1 has nothing else to read it off.
#[test]
fn a_seat_in_a_payment_window_is_told_what_it_owes() {
    use baylee_core::mana::ManaCost;

    let payer = PlayerId::new(0);
    let bystander = PlayerId::new(1);
    let mut session = Session::new(&a_table_with_a_warded_creature()).expect("preset builds");

    assert!(
        drive_into_ward_s_payment_window(&mut session, payer),
        "the removal was never cast at the warded creature"
    );

    let owed = baylee_view::ManaPayment::Fixed(ManaCost::from_symbol_generic(2));
    assert_eq!(
        session.awaiting_seat(),
        Some(payer),
        "the payer is the seat holding priority inside its own window"
    );
    assert_eq!(
        seat_view(&session, payer).owed,
        Some(owed),
        "a seat that has just agreed to pay is owed the number it agreed to"
    );
    assert_eq!(
        seat_view(&session, bystander).owed,
        Some(owed),
        "and so is the seat watching, which has no question to read it off"
    );

    // The number is usable, which is the whole claim: two taps and a pass
    // settle the tax, the window closes and the spell resolves.
    for _ in 0..4 {
        let Pending::Priority { player, legal } = session.pending().clone() else {
            break;
        };
        let Some(&source) = legal.mana_abilities.first() else {
            break;
        };
        session
            .act(player, PlayerAction::ActivateManaAbility { source })
            .expect("the window offers these and nothing else");
    }
    assert_eq!(
        seat_view(&session, payer).owed,
        Some(owed),
        "the total that was asked, not a remainder that shrinks as lands tap"
    );
    let _ = session.act(payer, PlayerAction::PassPriority);
    assert_eq!(
        seat_view(&session, bystander).owed,
        None,
        "a closed window owes nothing"
    );

    // Closing the window settles the tax; the spell under it is still on
    // the stack and resolves on the next round of passes.
    let throne = baylee_cards::decks::by_name("Roaming Throne").expect("compiled");
    for _ in 0..8 {
        let Some(seat) = session.awaiting_seat() else {
            break;
        };
        if !matches!(session.pending(), Pending::Priority { .. }) {
            break;
        }
        if session.act(seat, PlayerAction::PassPriority).is_err() {
            break;
        }
    }
    let state = session.state();
    assert!(
        !state
            .battlefield_view()
            .into_iter()
            .filter_map(|id| state.object(id))
            .any(|o| o.card.is_some_and(|c| c.index == throne)),
        "the tax was paid, so Swords to Plowshares resolved"
    );
}
