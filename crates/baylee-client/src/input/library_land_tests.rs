use super::*;
use baylee_client_core::{
    Interaction,
    test_support::{ViewBuilder, printed},
};
use baylee_engine::choice::{LegalActions, Pending, PlayerAction};

#[test]
fn a_revealed_library_land_click_uses_the_authoritative_offer() {
    let mut view = ViewBuilder::new(2).build();
    let top = printed(100, 0, "Forest", 1);
    let id = top.id;
    view.library_tops.push(top);
    let pending = |lands| Pending::Priority {
        player: view.seat,
        legal: Box::new(LegalActions {
            lands,
            ..Default::default()
        }),
    };
    let mut duel = Duel {
        interaction: Some(Interaction::new(pending(vec![id]), view.seat)),
        view: Some(view.clone()),
        ..Duel::default()
    };
    activate_card(&mut duel, id);
    assert!(matches!(duel.outbox.as_slice(), [PlayerAction::PlayLand { card }] if *card == id));
    duel.outbox.clear();
    duel.interaction = Some(Interaction::new(pending(vec![]), view.seat));
    activate_card(&mut duel, id);
    assert!(duel.outbox.is_empty(), "visible does not imply playable");
}
