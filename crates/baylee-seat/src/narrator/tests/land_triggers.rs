//! Taps whose lands set off a trigger: the standing orders and the seat's
//! taps together, as the bridge runs them (`WakeFilter::judge`, the mind's
//! `Seat::begin`, `WakeFilter::heard`), on a board rebuilt from an LLM game.

use super::*;
use crate::llm::seatstate::Seat;
use crate::narrator::{React, Until};
use crate::wake::{Orders, Verdict, WakeFilter};

/// Codex attempt 278, turn 18, q206: the first main phase of this seat (P2
/// there), with the opponent's Manabarbs on the table and Royal Assassin
/// in hand.
fn manabarbs_main_phase() -> (PlayerView, LegalActions) {
    let mut view = ViewBuilder::new(2)
        .with_battlefield(
            0,
            [
                card(73, "Island"),
                card(88, "Plains"),
                card(108, "Swamp"),
                card(112, "Swamp"),
                card(70, "Island"),
                card(121, "Yotian Soldier"),
            ],
        )
        .with_battlefield(1, [card(23, "Manabarbs"), card(9, "Goblin King")])
        .build();
    view.turn = 18;
    view.hand = vec![in_hand(100, "Royal Assassin")];
    // Royal Assassin is castable once paid (`payable`), as the engine
    // publishes it with the mana still in the lands.
    let legal = LegalActions {
        can_pass: true,
        mana_abilities: vec![id(73), id(88), id(108), id(112), id(70)],
        payable: vec![id(100)],
        ..LegalActions::default()
    };
    (view, legal)
}

fn asked(view: &PlayerView, legal: &LegalActions) -> Request {
    request(
        view.clone(),
        Pending::Priority {
            player: ME,
            legal: Box::new(legal.clone()),
        },
        LogTail::default(),
    )
}

/// The land `source` tapped for its mana, and Manabarbs' trigger for it on
/// the stack (CR 603.3: put there before this seat has priority again).
fn tap(view: &mut PlayerView, legal: &mut LegalActions, source: ObjectId, slot: u32) {
    let land = view
        .battlefield
        .iter_mut()
        .find(|object| object.id == source)
        .expect("a land of the seat's");
    land.status = ObjectStatus::TAPPED;
    let pool = &mut view.seats[0].mana_pool;
    match land.name.as_str() {
        "Swamp" => pool.black += 1,
        "Island" => pool.blue += 1,
        _ => pool.white += 1,
    }
    legal.mana_abilities.retain(|&id| id != source);
    let mut trigger = token(slot, 1, "Manabarbs", 0, 0);
    trigger.stack_item = Some(baylee_view::StackItem::Ability {
        source: id(23),
        ability: None,
        text: None,
        rules: None,
        token: None,
    });
    view.stack.push(trigger);
}

/// NEW-07: a land's trigger does not end a payment. After the last tap the
/// seat passes so that Manabarbs' triggers resolve (Royal Assassin is a
/// creature, and cannot be cast over them), and every window after that,
/// in the same step with the mana still in the pool, is the payment's next
/// step: the cast follows once the stack is empty. Before, the pass ended
/// the payment, the next windows were passed as nothing to do and, under
/// the model's `until`, as a quiet one, and the mana emptied with the step.
#[test]
fn a_land_trigger_from_the_taps_does_not_end_the_payment() {
    let (mut view, mut legal) = manabarbs_main_phase();
    let mut filter = WakeFilter::default();
    let first = asked(&view, &legal);
    assert_eq!(
        filter.judge(&first.view, &first.pending, &[]),
        Verdict::Wake(crate::wake::Why::RailStop)
    );
    let mut seat = Seat::new(&first.context, None, Style::default());
    let mut narrator = Narrator::new(&first.context);
    let wake = narrator.wake(&first, &[], None);
    let pick = wake
        .text
        .lines()
        .find(|line| line.contains("Cast Royal Assassin") && line.contains("(taps"))
        .and_then(|line| line.split_whitespace().next())
        .unwrap_or_else(|| panic!("Royal Assassin is reachable:\n{}", wake.text))
        .to_string();
    let resolved = wake
        .menu
        .resolve(&Decision {
            ask: Some("q12".into()),
            pick: vec![pick],
            ..Decision::default()
        })
        .expect("the cast resolves");
    let mut action = seat.keep(&first, narrator, resolved, None);
    // The model's answer at q206 also said `until: my_turn`.
    filter.apply_orders(
        None,
        Some(Orders {
            until: Some(Until::MyTurn),
            react: React::TargetsMe,
        }),
        0,
        &view,
        &[],
    );
    filter.heard(&view, &action);
    let mut slot = 150;
    // The three taps, each answered by the taps in flight.
    while let PlayerAction::ActivateManaAbility { source } = action {
        tap(&mut view, &mut legal, source, slot);
        slot += 1;
        let next = asked(&view, &legal);
        assert_eq!(
            filter.judge(&next.view, &next.pending, &[]),
            Verdict::Continue
        );
        action = seat.begin(&next, |_| false).expect("the taps go on").action;
        filter.heard(&view, &action);
    }
    assert_eq!(view.stack.len(), 3, "a trigger for each land tapped");
    assert_eq!(
        action,
        PlayerAction::PassPriority,
        "a creature cannot be cast over the triggers: they resolve first"
    );
    // Each trigger resolves in turn, and priority comes back in the same
    // step with the pool still full.
    while !view.stack.is_empty() {
        view.stack.pop();
        if view.stack.is_empty() {
            legal.castable = vec![id(100)];
        }
        let next = asked(&view, &legal);
        assert_eq!(
            filter.judge(&next.view, &next.pending, &[]),
            Verdict::Continue,
            "{} triggers left on the stack",
            view.stack.len()
        );
        action = seat
            .begin(&next, |_| false)
            .expect("the payment goes on")
            .action;
        filter.heard(&view, &action);
    }
    assert_eq!(action, PlayerAction::CastSpell { card: id(100) });
}
