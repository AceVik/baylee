//! What the model offers the player: the cards in hand that are lit and the permanents that may be activated. The hand's order belongs here because the only thing that ever disturbed it was a sort by playability — the view sends the order the cards were drawn in, the model must not touch it, and playability is carried by light alone rather than by position. The other half is what a card standing for several permanents may claim: a group is activatable only when every permanent in it is, or the player clicks it and is told no. Whether those permanents merged at all is `lanes`, and who may act at this moment is `seats`.

#[allow(clippy::wildcard_imports)] // this module's own vocabulary
use super::*;

#[test]
fn the_hand_keeps_the_order_the_cards_arrived_in() {
    // The view's hand is the engine's zone list, which a drawn card is
    // pushed onto — so "the order the view sends" is the order they were
    // drawn, and the model must not touch it. This test used to assert the
    // opposite (playable first, then cheapest); the sort it checked is
    // what moved a card out from under the pointer every time a land
    // untapped.
    let view = ViewBuilder::new(2)
        .with_hand(vec![
            ("Expensive Thing", 7, 100),
            ("Cheap Thing", 1, 101),
            ("Playable Thing", 5, 102),
        ])
        .build();
    let playable: HashSet<ObjectId> = [ObjectId::new(102, 0)].into_iter().collect();
    let m = BoardModel::from_view(
        &view,
        Openings {
            playable: &playable,
            reachable: &HashSet::new(),
            activatable: &HashSet::new(),
            proposed: &HashMap::new(),
        },
        &[],
        Registry::none(),
    );
    let names: Vec<&str> = m.hand.iter().map(|c| c.name.as_str()).collect();
    assert_eq!(
        names,
        ["Expensive Thing", "Cheap Thing", "Playable Thing"],
        "the hand was re-ordered"
    );
    // Playability is still reported — it is now carried by light alone.
    assert!(m.hand[2].playable);
    assert!(!m.hand[0].playable);
}

#[test]
fn a_group_is_activatable_only_when_every_card_in_it_is() {
    let objs = vec![token(1, 0, "Forest", 0, 0), token(2, 0, "Forest", 0, 0)];
    let view = ViewBuilder::new(2).with_battlefield(0, objs).build();

    let both: HashSet<ObjectId> = [ObjectId::new(1, 0), ObjectId::new(2, 0)]
        .into_iter()
        .collect();
    let one: HashSet<ObjectId> = std::iter::once(ObjectId::new(1, 0)).collect();
    let empty = HashSet::new();
    let unproposed = HashMap::new();

    let openings = |set| Openings {
        playable: &empty,
        reachable: &empty,
        activatable: set,
        proposed: &unproposed,
    };

    let lit = BoardModel::from_view(&view, openings(&both), &[], Registry::none());
    let group = &lit.pods[0].lanes[0].groups[0];
    assert_eq!(group.count(), 2, "identical permanents still merge");
    assert!(group.activatable);

    // One of the two cannot be tapped, so the card standing for both must
    // not claim it can — the player would click it and be told no.
    let half = BoardModel::from_view(&view, openings(&one), &[], Registry::none());
    assert!(!half.pods[0].lanes[0].groups[0].activatable);

    let dark = BoardModel::from_view(&view, openings(&empty), &[], Registry::none());
    assert!(!dark.pods[0].lanes[0].groups[0].activatable);
}
