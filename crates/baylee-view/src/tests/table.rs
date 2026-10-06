use super::*;

#[test]
fn opponents_are_seated_in_turn_order_from_the_viewing_seat() {
    let mut v = view(4);
    v.seat = PlayerId::new(2);
    let ring: Vec<u8> = v
        .opponents_in_turn_order()
        .into_iter()
        .map(PlayerId::get)
        .collect();
    // Seat 2 looks left to 3, then wraps to 0 and 1.
    assert_eq!(ring, vec![3, 0, 1]);
}

#[test]
fn opponent_ring_is_empty_in_a_one_seat_game() {
    let v = view(1);
    assert!(v.opponents_in_turn_order().is_empty());
}

#[test]
fn combat_reports_unblocked_attackers() {
    let mut v = view(2);
    let att = ObjectId::new(10, 0);
    let other = ObjectId::new(11, 0);
    v.combat.attackers = vec![
        AttackerView {
            creature: att,
            defending: Defender::Player(PlayerId::new(1)),
            blocked: true,
        },
        AttackerView {
            creature: other,
            defending: Defender::Player(PlayerId::new(1)),
            blocked: false,
        },
    ];
    v.combat.blockers = vec![BlockerView {
        blocker: ObjectId::new(20, 0),
        attacker: att,
    }];
    assert!(v.combat.is_active());
    assert!(!v.combat.is_unblocked(att));
    assert!(v.combat.is_unblocked(other));
    assert_eq!(v.combat.blockers_of(att).count(), 1);
}

/// CR 509.1h: an attacker stays blocked when its blockers leave, and the
/// two questions a client asks about it stop agreeing.
///
/// This is the shape a blink makes — the engine removes the departing
/// creature from combat and leaves the flag standing — and before the
/// flag reached the view it was unrepresentable: the blocker list is
/// empty, so "is it unblocked" answered yes and the whole squad's damage
/// was counted against a player none of it reaches.
#[test]
fn an_attacker_whose_blockers_have_gone_is_still_blocked() {
    let mut v = view(2);
    let att = ObjectId::new(10, 0);
    v.combat.attackers = vec![AttackerView {
        creature: att,
        defending: Defender::Player(PlayerId::new(1)),
        blocked: true,
    }];

    assert_eq!(
        v.combat.blockers_of(att).count(),
        0,
        "nothing is blocking it any more"
    );
    assert!(
        !v.combat.is_unblocked(att),
        "and it is blocked all the same, dealing its damage to nobody"
    );
}

#[test]
fn view_serialises_and_deserialises_unchanged() {
    let mut v = view(2);
    v.seats[0].no_max_hand_size = true;
    v.battlefield = vec![obj(1, 0), obj(2, 1)];
    let json = serde_json::to_vec(&v).expect("serialises");
    let back: PlayerView = serde_json::from_slice(&json).expect("deserialises");
    assert_eq!(v, back);
}

#[test]
fn older_seat_payloads_default_to_a_normal_hand_limit() {
    let mut json = serde_json::to_value(&view(2).seats[0]).unwrap();
    json.as_object_mut().unwrap().remove("no_max_hand_size");
    let seat: SeatView = serde_json::from_value(json).unwrap();
    assert!(!seat.no_max_hand_size);
}

#[test]
fn objects_are_found_across_every_zone() {
    let mut v = view(2);
    v.battlefield = vec![obj(1, 0)];
    v.graveyards[1] = vec![obj(2, 1)];
    assert!(v.object(ObjectId::new(1, 0)).is_some());
    assert!(v.object(ObjectId::new(2, 0)).is_some());
    assert!(v.object(ObjectId::new(99, 0)).is_none());
}
