use super::*;

#[test]
fn a_foretell_offer_is_named_and_resolves_to_the_special_action() {
    let (mut view, _) = board();
    view.hand = vec![in_hand(90, "Doomskar")];
    view.battlefield.clear();
    view.seats[0].mana_pool.colorless = 2;
    let pending = Pending::Priority {
        player: ME,
        legal: Box::new(LegalActions {
            can_pass: true,
            foretellable: vec![id(90)],
            ..Default::default()
        }),
    };
    let request = request(view, pending, LogTail::default());
    let wake = Narrator::new(&request.context).wake(&request, &[], None);
    assert!(
        wake.text.contains("Foretell Doomskar #90 for {2}"),
        "{}",
        wake.text
    );
    let answer = wake
        .menu
        .resolve(&Decision {
            ask: Some("q12".into()),
            pick: vec!["a1".into()],
            ..Default::default()
        })
        .unwrap();
    assert!(matches!(answer.act, Act::Now(PlayerAction::Foretell { card }) if card == id(90)));
}

/// Bug hunt of 3.11.2026, game 1, q65: Starnheim Unleashed was not offered
/// for foretell with untapped lands, only the {2} floating counted. The
/// engine's hint (`unpaid_foretell`) is offered as taps, then the action.
#[test]
fn a_foretell_the_lands_can_pay_is_offered_with_its_taps() {
    let mut view = ViewBuilder::new(2)
        .with_battlefield(0, [card(70, "Plains"), card(71, "Plains")])
        .build();
    view.hand = vec![in_hand(90, "Doomskar")];
    let pending = Pending::Priority {
        player: ME,
        legal: Box::new(LegalActions {
            can_pass: true,
            mana_abilities: vec![id(70), id(71)],
            unpaid_foretell: vec![id(90)],
            ..Default::default()
        }),
    };
    let request = request(view, pending, LogTail::default());
    let wake = Narrator::new(&request.context).wake(&request, &[], None);
    let line = wake
        .text
        .lines()
        .find(|line| line.contains("Foretell Doomskar #90 for {2}"))
        .unwrap_or_else(|| panic!("the foretell is offered:\n{}", wake.text));
    assert!(line.contains("(taps "), "{line}");
    let pick = line.split_whitespace().next().unwrap().to_string();
    let answer = wake
        .menu
        .resolve(&Decision {
            ask: Some("q12".into()),
            pick: vec![pick],
            ..Default::default()
        })
        .unwrap();
    assert!(
        matches!(&answer.act, Act::Taps { steps, then: PlayerAction::Foretell { card } }
            if *card == id(90) && steps.len() == 2),
        "{:?}",
        answer.act
    );
}
