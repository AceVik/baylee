//! Feedback triage regressions for the narrator and planned actions.

use super::*;

#[test]
fn looking_at_lists_all_inspected_cards_past_twelve_while_graveyard_keeps_twelve_cap() {
    let (mut view, log) = board();
    view.looking_at = (200..216).map(|slot| card(slot, "Forest")).collect();
    view.graveyards[0] = (300..316).map(|slot| card(slot, "Shock")).collect();
    let pending = Pending::ChooseCards {
        player: ME,
        options: (200..216).map(id).collect(),
        min: 0,
        max: 1,
        prompt: baylee_engine::choice::ChoicePrompt::SearchLibrary,
        total: None,
    };
    let request = request(view, pending, log);
    let wake = Narrator::new(&request.context).wake(&request, &[], None);
    let looking_line = wake
        .text
        .lines()
        .find(|l| l.starts_with("You are looking at (16):"))
        .expect("looking_at line");
    assert!(
        !looking_line.contains("older"),
        "looking_at must list every inspected card without truncating: {looking_line}"
    );
    for slot in 200..216 {
        assert!(
            looking_line.contains(&format!("#{slot}")),
            "missing #{slot} in {looking_line}"
        );
    }
    let gy_line = wake
        .text
        .lines()
        .find(|l| l.starts_with("Your graveyard (16):"))
        .expect("graveyard line");
    assert!(
        gy_line.contains("4 older"),
        "graveyard line still caps at 12 names: {gy_line}"
    );
    assert!(
        wake.text.contains("pick=[]"),
        "optional ChooseCards (min == 0) must document pick=[] for choosing none: {}",
        wake.text
    );
}

#[test]
fn number_choice_names_the_top_stack_item() {
    let (mut view, log) = board();
    let mut stack_obj = card(88, "Serra Angel");
    stack_obj.stack_item = Some(baylee_view::StackItem::Spell);
    view.stack.push(stack_obj);
    let num_req = request(
        view,
        Pending::ChooseNumber {
            player: ME,
            min: 0,
            max: 3,
            reason: baylee_engine::choice::NumberPrompt::Choice,
        },
        log,
    );
    let wake = Narrator::new(&num_req.context).wake(&num_req, &[], None);
    assert!(
        wake.text.contains("Choose a number for Serra Angel #88"),
        "NumberPrompt::Choice should name the stack item: {}",
        wake.text
    );
}

#[test]
fn fixed_payment_notes_mana_already_in_the_pool() {
    let (mut view, _) = board();
    view.owed = Some(baylee_core::mana::ManaPayment::Fixed(
        "{1}{G}".parse().unwrap(),
    ));
    view.seats[0].mana_pool.green = 1;
    let pay_req = request(view.clone(), priority(&view), LogTail::default());
    let wake = Narrator::new(&pay_req.context).wake(&pay_req, &[], None);
    assert!(
        wake.text.contains("in your pool"),
        "fixed payment prompt with floating mana should note pool mana: {}",
        wake.text
    );
}

#[test]
fn an_activation_plan_resolves_the_pretapped_unpaid_ability() {
    let (mut view, _) = board();
    view.battlefield.push(card(75, "Jayemdae Tome"));
    let mut pending = priority(&view);
    if let Pending::Priority { legal, .. } = &mut pending {
        legal
            .unpaid_abilities
            .push((id(75), 0, "{4}".parse().unwrap()));
    }
    let act_req = request(view, pending, LogTail::default());
    let menu = Narrator::new(&act_req.context).menu(&act_req);
    let step = Step::parse("activate #75").expect("valid step");
    let decision = menu
        .plan_decision(&step, &act_req)
        .expect("Step::Activate should match pre-tapped Act::Taps ability");
    let resolved = menu.resolve(&decision).expect("resolves");
    assert!(matches!(
        resolved.act,
        Act::Taps {
            then: PlayerAction::ActivateAbility {
                source,
                ability_index: 0
            },
            ..
        } if source == id(75)
    ));
}

#[test]
fn follow_plan_passes_triggered_abilities_from_land_taps_and_suppresses_false_undone_while_owed() {
    use crate::llm::seatstate::Seat;

    // Part 1: Tapping a land to cast a sorcery triggers an ability (e.g. Manabarbs / Lifetap)
    // onto the stack. `follow_plan` must pass priority while only abilities are on the stack,
    // and then cast the sorcery once the stack clears.
    let (view, log) = board();
    let req0 = request(view.clone(), priority(&view), log);
    let mut seat = Seat::new(&req0.context, None, Style::default());
    let mut narrator = Narrator::new(&req0.context);
    let wake = narrator.wake(&req0, &[], None);
    // Pick the option that taps two lands and casts Rampant Growth (#53).
    let resolved = wake
        .menu
        .resolve(&Decision {
            ask: Some("q12".into()),
            pick: vec!["a4".into()],
            ..Decision::default()
        })
        .expect("Rampant Growth tap plan");
    assert!(matches!(
        resolved.act,
        Act::Taps {
            then: PlayerAction::CastSpell { card },
            ..
        } if card == id(53)
    ));
    let first_action = seat.keep(&req0, narrator, resolved, None);
    assert!(matches!(
        first_action,
        PlayerAction::ActivateManaAbility { .. }
    ));

    // Second tap completes, and puts a triggered ability on the stack!
    let (mut view2, _) = board();
    view2.seats[0].mana_pool.green = 1;
    let mut p2 = priority(&view2);
    if let Pending::Priority { legal, .. } = &mut p2 {
        legal.mana_abilities = vec![id(22)];
    }
    let req1 = request(view2, p2, LogTail::default());
    let second_tap = seat
        .begin(&req1, |_| false)
        .expect("second tap from follow_plan");
    assert_eq!(
        second_tap.action,
        PlayerAction::ActivateManaAbility { source: id(22) }
    );

    // Now the stack has a triggered ability (StackItem::Ability), so Rampant Growth (#53)
    // is temporarily NOT in legal.castable!
    let (mut view3, _) = board();
    view3.seats[0].mana_pool.green = 2;
    let mut trigger_obj = token(99, 1, "Manabarbs", 0, 0);
    trigger_obj.stack_item = Some(baylee_view::StackItem::Ability {
        source: id(99),
        ability: None,
        text: None,
        rules: None,
        token: None,
    });
    view3.stack = vec![trigger_obj];
    let mut p3 = priority(&view3);
    if let Pending::Priority { legal, .. } = &mut p3 {
        legal.mana_abilities.clear();
        legal.castable.clear();
    }
    let req2 = request(view3, p3, LogTail::default());
    let pass_on_trigger = seat
        .begin(&req2, |_| false)
        .expect("follow_plan must pass priority while triggered ability from tap is on the stack");
    assert_eq!(pass_on_trigger.action, PlayerAction::PassPriority);

    // Trigger resolves, stack is empty, Rampant Growth (#53) is now in legal.castable!
    let (mut view4, _) = board();
    view4.seats[0].mana_pool.green = 2;
    let mut p4 = priority(&view4);
    if let Pending::Priority { legal, .. } = &mut p4 {
        legal.mana_abilities.clear();
        legal.castable = vec![id(53)];
    }
    let req3 = request(view4, p4, LogTail::default());
    let cast_action = seat
        .begin(&req3, |_| false)
        .expect("follow_plan must cast Rampant Growth once trigger clears");
    assert_eq!(cast_action.action, PlayerAction::CastSpell { card: id(53) });

    // Part 2: `undone` must NOT emit a false "did not happen" warning during a deferred
    // payment window (`view.owed.is_some()`), and MUST emit it if the cast is rolled back
    // to hand after the payment window closes (`view.owed.is_none()`).
    seat.casting = Some(id(53));
    let (mut view_owed, _) = board();
    view_owed.owed = Some(baylee_core::mana::ManaPayment::Fixed(
        "{1}{G}".parse().unwrap(),
    ));
    let req_owed = request(view_owed.clone(), priority(&view_owed), LogTail::default());
    assert!(seat.begin(&req_owed, |_| false).is_none());
    assert!(
        seat.told(false).is_empty(),
        "must not claim cast did not happen while payment window (view.owed) is open"
    );

    // Now payment fails and the window closes with #53 still in hand and no LogEvent::Cast.
    let (view_rolled_back, _) = board();
    let req_rolled_back = request(
        view_rolled_back.clone(),
        priority(&view_rolled_back),
        LogTail::default(),
    );
    assert!(seat.begin(&req_rolled_back, |_| false).is_none());
    let notes = seat.told(false);
    assert!(
        notes
            .iter()
            .any(|n| n.contains("Your cast of Rampant Growth #53 did not happen")),
        "expected rollback note after payment window closed without cast, got {notes:?}"
    );
}
