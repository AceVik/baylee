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
