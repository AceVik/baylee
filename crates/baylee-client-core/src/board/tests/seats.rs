//! The seat a pod is drawn for, and the line of numbers beside its mat. Pods are ordered local seat first and then clockwise in turn order whatever seat the view belongs to, and priority is reported on the one seat holding it or on none. The rest is the reading a player would otherwise do by hand with eight opponents and thirty seconds: the token populations counted as text, the power that can actually swing as against everything that is merely a creature, and what is left to block in the air. What stands in a lane and how it grouped is `lanes`; the piles beside the mat are `piles`.

#[allow(clippy::wildcard_imports)] // this module's own vocabulary
use super::*;

#[test]
fn token_chips_summarise_a_wide_board_as_text() {
    let mut objs: Vec<PublicObject> = (0..12).map(|i| token(i, 0, "Soldier", 1, 1)).collect();
    objs[0].status = ObjectStatus::TAPPED;
    objs.extend((20..23).map(|i| {
        let mut t = token(i, 0, "Treasure", 0, 0);
        t.types = TypeSet::ARTIFACT;
        t.power = None;
        t.toughness = None;
        t
    }));
    let view = ViewBuilder::new(2).with_battlefield(0, objs).build();
    let m = model(&view);
    let pod = m.pod(PlayerId::new(0)).expect("pod");

    assert_eq!(pod.tokens.len(), 2);
    assert_eq!(pod.tokens[0].label(), "12× 1/1 Soldier");
    assert_eq!(pod.tokens[0].tapped, 1);
    assert_eq!(pod.tokens[1].label(), "3× Treasure");
}

#[test]
fn threat_summary_counts_what_can_actually_swing() {
    let mut objs = vec![
        token(1, 0, "Bear", 2, 2),
        token(2, 0, "Bear", 2, 2),
        token(3, 0, "Wall", 0, 4),
        token(4, 0, "Bear", 2, 2),
    ];
    objs[1].status = ObjectStatus::TAPPED;
    objs[2].keywords = keyword_bits::DEFENDER;
    objs[3].summoning_sick = true;
    let mut land = token(5, 0, "Forest", 0, 0);
    land.types = TypeSet::LAND;
    land.power = None;
    land.toughness = None;
    objs.push(land);

    let view = ViewBuilder::new(2).with_battlefield(0, objs).build();
    let m = model(&view);
    let t = m.pod(PlayerId::new(0)).expect("pod").threat;

    // Only the one untapped, non-sick, non-defender bear can attack now.
    assert_eq!(t.attack_power, 2);
    // Untapped and not a defender: the ready bear and the sick one.
    assert_eq!(t.potential_attackers, 2);
    // Blockers include the wall.
    assert_eq!(t.potential_blockers, 3);
    assert_eq!(t.open_mana, 1);
}

#[test]
fn air_defence_counts_flying_and_reach() {
    let mut objs = vec![
        token(1, 0, "Bird", 1, 1),
        token(2, 0, "Spider", 1, 3),
        token(3, 0, "Bear", 2, 2),
    ];
    objs[0].keywords = keyword_bits::FLYING;
    objs[1].keywords = keyword_bits::REACH;
    let view = ViewBuilder::new(2).with_battlefield(0, objs).build();
    let m = model(&view);
    assert_eq!(m.pod(PlayerId::new(0)).expect("pod").threat.air_defence, 2);
}

#[test]
fn pods_are_ordered_local_first_then_clockwise_in_turn_order() {
    let mut view = ViewBuilder::new(4).build();
    view.seat = PlayerId::new(2);
    let m = model(&view);
    let order: Vec<u8> = m.pods.iter().map(|p| p.player.get()).collect();
    assert_eq!(order, vec![2, 3, 0, 1]);
    assert!(m.pods[0].is_local);
    assert!(!m.pods[1].is_local);
}

#[test]
fn the_awaited_seat_is_the_one_reported_on_the_pod() {
    let view = ViewBuilder::new(3).with_awaiting(Some(2)).build();
    let m = model(&view);
    assert!(!m.pod(PlayerId::new(0)).expect("pod").has_priority);
    assert!(m.pod(PlayerId::new(2)).expect("pod").has_priority);

    let nobody = ViewBuilder::new(3).with_awaiting(None).build();
    let m = model(&nobody);
    assert!(m.pods.iter().all(|p| !p.has_priority));
}
