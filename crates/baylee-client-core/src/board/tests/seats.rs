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
    assert!(!m.pod(PlayerId::new(0)).expect("pod").is_awaited);
    assert!(m.pod(PlayerId::new(2)).expect("pod").is_awaited);

    let nobody = ViewBuilder::new(3).with_awaiting(None).build();
    let m = model(&nobody);
    assert!(m.pods.iter().all(|p| !p.is_awaited));
}

/// Before turn 1 every seat decides its opening hand at once (#257), and a
/// view's `awaiting` names only its own seat, and only until it keeps. Every
/// seat in `deciding` is waited on then, whatever `awaiting` says; from
/// turn 1 on `deciding` is empty and `awaiting` alone decides again.
#[test]
fn every_seat_still_deciding_its_hand_is_awaited() {
    let awaited = |view: &PlayerView| -> Vec<u8> {
        let mut seats: Vec<u8> = model(view)
            .pods
            .iter()
            .filter(|p| p.is_awaited)
            .map(|p| p.player.get())
            .collect();
        seats.sort_unstable();
        seats
    };
    let mut view = ViewBuilder::new(3).with_awaiting(Some(0)).build();
    view.deciding = [0, 1, 2].map(PlayerId::new).into_iter().collect();
    assert_eq!(awaited(&view), [0, 1, 2], "nobody has kept yet");

    view.awaiting = None;
    view.deciding = [1, 2].map(PlayerId::new).into_iter().collect();
    assert_eq!(
        awaited(&view),
        [1, 2],
        "this seat has kept and its view names nobody, but two seats are still deciding"
    );

    view.awaiting = Some(PlayerId::new(1));
    view.deciding = baylee_core::ids::SeatSet::new();
    assert_eq!(awaited(&view), [1], "turn 1: the one seat asked");
}

/// Who is answering for a chair comes off the roster, not the view.
///
/// The three states a chair can be in, each read from the payload that
/// actually carries them: a present human, the house playing a chair by
/// arrangement, and a player's chair the house is holding while nobody is on
/// the other end of it. `SeatIdentity` keeps the last two apart on the wire
/// so a thirty-second hiccup does not rename the chair to the house, and this
/// is what stops that distinction being thrown away on arrival.
#[test]
fn a_chair_says_who_is_answering_for_it() {
    let view = ViewBuilder::new(3).build();
    let roster = vec![
        identity(0, false, false),
        identity(1, true, false),
        identity(2, false, true),
    ];
    let m = BoardModel::from_view(&view, Openings::none(), |_| WIDE, &roster, Registry::none());
    let role = |seat: u8| m.pod(PlayerId::new(seat)).expect("pod").role;
    assert_eq!(role(0), SeatRole::Present, "somebody is sitting there");
    assert_eq!(role(1), SeatRole::House, "the house plays that chair");
    assert_eq!(role(2), SeatRole::Away, "and that one is being held");

    // The half that matters to whatever is about to be drawn: two of the
    // three answer the next question with the house, and they are still not
    // the same state.
    assert!(!role(0).answered_by_the_house());
    assert!(role(1).answered_by_the_house() && role(2).answered_by_the_house());
    assert_ne!(role(1), role(2), "an arrangement is not an interruption");
}

/// A table nobody has been introduced at is a table of present players.
///
/// The roster is a second payload — sent once per socket, where a view
/// arrives many times a turn — so the first frame of a game is drawn without
/// it, and so is every frame of the offline harness. The counter-half is what
/// makes the claim worth anything: the same view with a roster does say
/// something, so "everyone is present" is an answer to an empty roster rather
/// than to the question never being asked.
#[test]
fn an_empty_roster_seats_nobody_the_house_is_playing_for() {
    let view = ViewBuilder::new(2).build();
    let bare = BoardModel::from_view(&view, Openings::none(), |_| WIDE, &[], Registry::none());
    assert!(
        bare.pods.iter().all(|pod| pod.role == SeatRole::Present),
        "a chair nothing has been said about belongs to a player"
    );

    let roster = vec![identity(0, false, false), identity(1, true, false)];
    let told = BoardModel::from_view(&view, Openings::none(), |_| WIDE, &roster, Registry::none());
    assert!(
        told.pods.iter().any(|pod| pod.role == SeatRole::House),
        "with a roster the same view does say who is at the table, so the \
         bare one above is answering a question it was asked"
    );
}

/// A roster that breaks its own rule still gives one of the three answers.
///
/// `SeatIdentity` documents `is_ai` and `away` as never both set, and that is
/// a promise made by a payload this crate does not build. Reading them as an
/// ordered pair of questions rather than as a match over both is what keeps a
/// fourth state out of the client: the chair is *held*, which is the more
/// urgent of the two and the only one of them that can stop being true.
#[test]
fn a_chair_that_is_both_is_read_as_the_one_that_can_end() {
    let view = ViewBuilder::new(2).build();
    let roster = vec![identity(0, false, false), identity(1, true, true)];
    let m = BoardModel::from_view(&view, Openings::none(), |_| WIDE, &roster, Registry::none());
    assert_eq!(m.pod(PlayerId::new(1)).expect("pod").role, SeatRole::Away);
}
